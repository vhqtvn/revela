# Helm linter.

import logging
import re
from typing import Tuple
from pathlib import Path

import click

from forge import LocalShell
from applogging import logger, init_logging


@click.group()
@click.option(
    "--log-metadata/--no-log-metadata",
    default=True,
)
@logger
def main(log_metadata: bool) -> None:
    init_logging(logger=log, level=logging.DEBUG, print_metadata=log_metadata)


@main.command()
@click.argument("paths", nargs=-1)
@logger
def helm(paths: Tuple[str]) -> None:
    shell = LocalShell()

    error = False
    for path in paths:
        result = shell.run(["helm", "lint", path])
        for line in result.output.decode().splitlines():
            if line.startswith("[ERROR]"):
                match = re.match(
                    r".ERROR. (?P<section>[^:]+?): (?P<error_type>.*) at [(](?P<filename>.*):(?P<line>\d+)[)]: (?P<message>.*)",
                    line,
                )
                if match:
                    fullpath = Path(path).parent / match.group("filename")
                    log.error(
                        "::error file={fullpath},line={line},col=1::{message}".format(
                            fullpath=fullpath, **match.groupdict()
                        )
                    )
                    error = True

    if error:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
