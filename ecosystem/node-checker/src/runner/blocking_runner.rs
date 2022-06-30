// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use super::{Runner, RunnerError};
use crate::{
    evaluator::EvaluationSummary,
    evaluators::{
        metrics::{parse_metrics, MetricsEvaluatorInput},
        system_information::SystemInformationEvaluatorInput,
        EvaluatorType,
    },
    metric_collector::MetricCollector,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use log::debug;
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct BlockingRunnerArgs {
    #[clap(long, default_value = "5")]
    pub metrics_fetch_delay_secs: u64,
}

#[derive(Debug)]
pub struct BlockingRunner<M: MetricCollector> {
    args: BlockingRunnerArgs,
    baseline_metric_collector: M,
    evaluators: Vec<EvaluatorType>,
}

impl<M: MetricCollector> BlockingRunner<M> {
    pub fn new(
        args: BlockingRunnerArgs,
        baseline_metric_collector: M,
        evaluators: Vec<EvaluatorType>,
    ) -> Self {
        Self {
            args,
            baseline_metric_collector,
            evaluators,
        }
    }

    fn parse_response(&self, lines: Vec<String>) -> Result<PrometheusScrape, RunnerError> {
        parse_metrics(lines)
            .context("Failed to parse metrics response")
            .map_err(RunnerError::ParseMetricsError)
    }

    async fn collect_metrics<MC: MetricCollector>(
        metric_collector: &MC,
    ) -> Result<Vec<String>, RunnerError> {
        metric_collector
            .collect_metrics()
            .await
            .map_err(RunnerError::MetricCollectorError)
    }
}

/// This runner doesn't block in the multithreading sense, but from the user
/// perspective. To run the health check, we pull metrics once, wait, and then
/// pull the metrics again. It does not support continually running beyond this
/// point. You can imagine smarter versions of this where you store the last seen
/// set of metrics, then compare against that, or perhaps even multiple previously
/// seen sets of metrics and do more complex analysis. Additionally we could leverage
/// things like long polling +/ sticky routing to make it that the client request
/// doesn't just hang waiting for the run to complete.
#[async_trait]
impl<M: MetricCollector> Runner for BlockingRunner<M> {
    async fn run<T: MetricCollector>(
        &self,
        target_collector: &T,
    ) -> Result<EvaluationSummary, RunnerError> {
        debug!("Collecting system information from baseline node");
        let baseline_system_information = self
            .baseline_metric_collector
            .collect_system_information()
            .await
            .map_err(RunnerError::MetricCollectorError)?;
        debug!("{:?}", baseline_system_information);

        debug!("Collecting system information from target node");
        let target_system_information = target_collector
            .collect_system_information()
            .await
            .map_err(RunnerError::MetricCollectorError)?;
        debug!("{:?}", target_system_information);

        debug!("Collecting first round of baseline metrics");
        let first_baseline_metrics = self
            .baseline_metric_collector
            .collect_metrics()
            .await
            .map_err(RunnerError::MetricCollectorError)?;

        debug!("Collecting first round of target metrics");
        let first_target_metrics = Self::collect_metrics(target_collector).await?;

        let first_baseline_metrics = self.parse_response(first_baseline_metrics)?;
        let first_target_metrics = self.parse_response(first_target_metrics)?;

        tokio::time::sleep(Duration::from_secs(self.args.metrics_fetch_delay_secs)).await;

        debug!("Collecting second round of baseline metrics");
        let second_baseline_metrics =
            Self::collect_metrics(&self.baseline_metric_collector).await?;

        debug!("Collecting second round of target metrics");
        let second_target_metrics = Self::collect_metrics(target_collector).await?;

        let second_baseline_metrics = self.parse_response(second_baseline_metrics)?;
        let second_target_metrics = self.parse_response(second_target_metrics)?;

        let mut evaluation_results = Vec::new();

        let metrics_evaluator_input = MetricsEvaluatorInput {
            previous_baseline_metrics: first_baseline_metrics,
            previous_target_metrics: first_target_metrics,
            latest_baseline_metrics: second_baseline_metrics,
            latest_target_metrics: second_target_metrics,
        };

        let system_information_evaluator_input = SystemInformationEvaluatorInput {
            baseline_system_information,
            target_system_information,
        };

        for evaluator in &self.evaluators {
            let mut local_evaluation_results = match evaluator {
                EvaluatorType::Metrics(evaluator) => evaluator
                    .evaluate(&metrics_evaluator_input)
                    .await
                    .map_err(RunnerError::MetricEvaluatorError)?,
                EvaluatorType::SystemInformation(evaluator) => evaluator
                    .evaluate(&system_information_evaluator_input)
                    .await
                    .map_err(RunnerError::SystemInformationEvaluatorError)?,
            };
            evaluation_results.append(&mut local_evaluation_results);
        }

        let complete_evaluation = EvaluationSummary::from(evaluation_results);

        Ok(complete_evaluation)
    }
}
