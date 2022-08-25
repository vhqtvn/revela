import json
import os
import unittest
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, OrderedDict, Sequence, Union

from click.testing import CliRunner
from .forge import (
    AwsError, FakeTime, ForgeFormatter, ForgeResult, ForgeState,
    Git, K8sForgeRunner, assert_aws_token_expiration, find_recent_images,
    format_pre_comment, get_dashboard_link, get_humio_logs_link,
    get_validator_logs_link, list_eks_clusters, list_jobs, main, ForgeContext, LocalForgeRunner, FakeShell,
    FakeFilesystem, RunResult, FakeProcesses
)


def get_cwd() -> Path:
    return Path(__file__).absolute().parent


def get_fixture_path(fixture_name: str) -> Path:
    return get_cwd() / "fixtures" / fixture_name


class AssertFixtureMixin:
    def assertFixture(self, test_str: str, fixture_name: str) -> None:
        fixture_path = get_fixture_path(fixture_name)
        if os.getenv("FORGE_WRITE_FIXTURES") == "true":
            print(f"Writing fixture to {str(fixture_path)}")
            fixture_path.write_text(test_str)
            fixture = test_str
        else:
            fixture = fixture_path.read_text()
        temp = Path(tempfile.mkstemp()[1])
        temp.write_text(test_str)
        self.assertMultiLineEqual(
            test_str,
            fixture,
            f"Fixture {fixture_name} does not match"
            "\n"
            f"Wrote to {str(temp)} for comparison"
            "\nRerun with FORGE_WRITE_FIXTURES=true to update the fixtures"
        )


class SpyShell(FakeShell):
    def __init__(self, command_map: Dict[str, Union[RunResult, Exception]]) -> None:
        self.command_map = command_map
        self.commands = []

    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        result = self.command_map.get(" ".join(command), super().run(command))
        self.commands.append(" ".join(command))
        if isinstance(result, Exception):
            raise result
        return result

    def assert_commands(self, testcase) -> None:
        testcase.assertEqual(list(self.command_map.keys()), self.commands)


class SpyFilesystem(FakeFilesystem):
    def __init__(self, expected_writes: Dict[str, bytes], expected_reads: Dict[str, bytes]) -> None:
        self.expected_writes = expected_writes
        self.expected_reads = expected_reads
        self.writes = {}
        self.reads = []
        self.temp_count = 1

    def write(self, filename: str, contents: bytes) -> None:
        self.writes[filename] = contents

    def read(self, filename: str) -> bytes:
        self.reads.append(filename)
        return self.expected_reads.get(filename, b"")

    def assert_writes(self, testcase) -> None:
        for filename, contents in self.expected_writes.items():
            testcase.assertIn(filename, self.writes, f"{filename} was not written: {self.writes}")
            testcase.assertMultiLineEqual(self.writes[filename].decode(), contents.decode(), f"{filename} did not match expected contents")

    def assert_reads(self, testcase) -> None:
        for filename in self.expected_reads.keys():
            testcase.assertIn(filename, self.reads, f"{filename} was not read")

    def mkstemp(self) -> str:
        filename = f"temp{self.temp_count}"
        self.temp_count += 1
        return filename


def fake_context(shell=None, filesystem=None, processes=None, time=None) -> ForgeContext:
    return ForgeContext(
        shell=shell if shell else FakeShell(),
        filesystem=filesystem if filesystem else FakeFilesystem(),
        processes=processes if processes else FakeProcesses(),
        time=time if time else FakeTime(),

        forge_test_suite="banana",
        local_p99_latency_ms_threshold="6000",
        forge_runner_tps_threshold="593943",
        forge_runner_duration_secs="123",

        reuse_args=[],
        keep_args=[],
        haproxy_args=[],

        aws_account_num="123",
        aws_region="banana-east-1",

        forge_image_tag="forge_asdf",
        image_tag="asdf",
        upgrade_image_tag="upgrade_asdf",
        forge_namespace="potato",
        forge_cluster_name="tomato",
        forge_blocking=True,

        github_actions="false",
        github_job_url="https://banana",
    )


class ForgeRunnerTests(unittest.TestCase):
    def testLocalRunner(self) -> None:
        shell = SpyShell(OrderedDict([
            (
                'cargo run -p forge-cli -- --suite banana --mempool-backlog 500'
                '0 --avg-tps 593943 --max-latency-ms 6000 --duration-secs 123 t'
                'est k8s-swarm --image-tag asdf --upgrade-image-tag upgrade_asd'
                'f --namespace potato --port-forward',
                RunResult(0, b"orange"),
            ),
        ]))
        filesystem = SpyFilesystem({}, {})
        context = fake_context(shell, filesystem)
        runner = LocalForgeRunner()
        result = runner.run(context)
        self.assertEqual(result.state, ForgeState.PASS, result.output)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)

    def testK8sRunner(self) -> None:
        self.maxDiff = None
        shell = SpyShell(OrderedDict([
            ("kubectl delete pod -n default -l forge-namespace=potato --force", RunResult(0, b"")),
            ("kubectl wait -n default --for=delete pod -l forge-namespace=potato", RunResult(0, b"")),
            ("kubectl apply -n default -f temp1", RunResult(0, b"")),
            ("kubectl wait -n default --timeout=5m --for=condition=Ready pod/potato-1659078000-asdf", RunResult(0, b"")),
            ("kubectl logs -n default -f potato-1659078000-asdf", RunResult(0, b"")),
            ("kubectl get pod -n default potato-1659078000-asdf -o jsonpath='{.status.phase}'", RunResult(0, b"Succeeded")),
        ]))
        forge_yaml = get_cwd() / "forge-test-runner-template.yaml"
        template_fixture = get_fixture_path("forge-test-runner-template.fixture")
        filesystem = SpyFilesystem({
            "temp1": template_fixture.read_bytes(),
        }, {
            "testsuite/forge-test-runner-template.yaml": forge_yaml.read_bytes(),
        })
        context = fake_context(shell, filesystem)
        runner = K8sForgeRunner()
        result = runner.run(context)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)
        self.assertEqual(result.state, ForgeState.PASS, result.output)


class TestAWSTokenExpiration(unittest.TestCase):
    def testNoAwsToken(self) -> None:
        with self.assertRaisesRegex(AwsError, "AWS token is required"): 
            assert_aws_token_expiration(None)

    def testAwsTokenExpired(self) -> None:
        expiration = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S%z")
        with self.assertRaisesRegex(AwsError, "AWS token has expired"):
            assert_aws_token_expiration(expiration)
    
    def testAwsTokenMalformed(self) -> None:
        with self.assertRaisesRegex(AwsError, "Invalid date format:.*"):
            assert_aws_token_expiration("asdlkfjasdlkjf")


class TestFindRecentImage(unittest.TestCase):
    def testFindRecentImage(self) -> None:
        shell = SpyShell(OrderedDict([
            ("git rev-parse HEAD~0", RunResult(0, b"potato\n")),
            ("aws ecr describe-images --repository-name aptos/validator --image-ids imageTag=potato", RunResult(1, b"")),
            ("git rev-parse HEAD~1", RunResult(0, b"lychee\n")),
            ("aws ecr describe-images --repository-name aptos/validator --image-ids imageTag=lychee", RunResult(0, b"")),
        ]))
        git = Git(shell)
        image_tags = find_recent_images(shell, git, 1)
        self.assertEqual(list(image_tags), ["lychee"])
        shell.assert_commands(self)

    def testDidntFindRecentImage(self) -> None:
        shell = SpyShell(OrderedDict([
            ("git rev-parse HEAD~0", RunResult(0, b"crab\n")),
            ("aws ecr describe-images --repository-name aptos/validator --image-ids imageTag=crab", RunResult(1, b"")),
        ]))
        git = Git(shell)
        with self.assertRaises(Exception):
            list(find_recent_images(shell, git, 1, commit_threshold=1))


class ForgeFormattingTests(unittest.TestCase, AssertFixtureMixin):
    maxDiff = None

    def testReport(self) -> None:
        filesystem = SpyFilesystem({"test": b"banana"}, {})
        context = fake_context(filesystem=filesystem)
        result = ForgeResult.from_args(ForgeState.PASS, "test")
        context.report(result, [
            ForgeFormatter("test", lambda c, r: "banana")
        ])
        filesystem.assert_reads(self)
        filesystem.assert_writes(self)

    def testHumioLogLink(self) -> None:
        self.assertFixture(
            get_humio_logs_link("forge-pr-2983"),
            "testHumioLogLink.fixture"
        )

    def testValidatorLogsLink(self) -> None:
        self.assertFixture(
            get_validator_logs_link("aptos-perry", "perrynet", True),
            "testValidatorLogsLink.fixture",
        )

    def testDashboardLink(self) -> None:
        self.assertFixture(
            get_dashboard_link(
                "aptos-forge-1",
                "forge-pr-2983",
                "forge-1",
                True,
            ),
            "testDashboardLink.fixture",
        )

    def testFormatPreComment(self) -> None:
        context = fake_context()
        self.assertFixture(
            format_pre_comment(context),
            "testFormatPreComment.fixture",
        )



class ForgeMainTests(unittest.TestCase, AssertFixtureMixin):
    maxDiff = None

    def testMain(self) -> None:
        runner = CliRunner()
        with runner.isolated_filesystem():
            os.mkdir(".git")
            os.mkdir("testsuite")
            template_name =  "forge-test-runner-template.yaml"
            Path(f"testsuite/{template_name}").write_text(
                (Path(__file__).parent / template_name).read_text()
            )
            result = runner.invoke(main, [
                "test", "--dry-run",
                "--forge-cluster-name", "forge-1",
                "--forge-report", "temp-report",
                "--forge-pre-comment", "temp-pre-comment",
                "--forge-comment", "temp-comment",
                "--github-step-summary", "temp-step-summary",
            ], catch_exceptions=False)
            self.assertEqual(
                sorted(os.listdir(".")),
                sorted(['temp-report', 'testsuite', 'temp-pre-comment', '.git', 'temp-comment', 'temp-step-summary']),
            )
            report = Path("temp-report").read_text()
            pre_comment = Path("temp-pre-comment").read_text()
            comment = Path("temp-comment").read_text()
            step_summary = Path("temp-comment").read_text()
        self.assertFixture(result.output, "testMain.fixture")
        self.assertFixture(pre_comment, "testMainPreComment.fixture")
        self.assertFixture(report, "testMainReport.fixture")
        self.assertFixture(comment, "testMainComment.fixture")
        self.assertFixture(step_summary, "testMainComment.fixture")


class TestListClusters(unittest.TestCase):
    def testListClusters(self) -> None:
        fake_clusters = (
            json.dumps({
                "clusters": [
                    "banana-fake-1",
                    "aptos-forge-banana-1",
                    "aptos-forge-potato-2",
                ],
            })
        )
        shell = SpyShell(OrderedDict([
            ("aws eks list-clusters", RunResult(0, fake_clusters.encode())),
        ]))
        clusters = list_eks_clusters(shell)
        self.assertEqual(clusters, ["aptos-forge-banana-1", "aptos-forge-potato-2"])
        shell.assert_commands(self)

    def testListClustersFails(self) -> None:
        with self.assertRaises(Exception):
            shell = SpyShell(OrderedDict([
                ("Blah", RunResult(0, b"")),
            ]))
            list_eks_clusters(shell)
            shell.assert_commands(self)