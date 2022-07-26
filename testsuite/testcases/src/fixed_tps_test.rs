// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, Test};
use tokio::time::Duration;

pub struct FixedTpsTest;

impl Test for FixedTpsTest {
    fn name(&self) -> &'static str {
        "fixed-tps-test"
    }
}

impl NetworkTest for FixedTpsTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = Duration::from_secs(240);
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // Generate some traffic with fixed tps 10
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, Some(10))?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat, duration);

        Ok(())
    }
}
