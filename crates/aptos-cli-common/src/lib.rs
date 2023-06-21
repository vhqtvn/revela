// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anstyle::{Color::Ansi, Style};
use clap::{builder::Styles, CommandFactory};
use std::path::Path;

/// A style for the CLI that closely resembles the Clap v3 color scheme
pub fn aptos_cli_style() -> Styles {
    Styles::styled()
        // Help headers
        // To test: `aptos help`
        .header(
            Style::new()
                .bold()
                .fg_color(Some(Ansi(anstyle::AnsiColor::Yellow))),
        )
        // The word Usage, which should match the help headers for consistency
        // To test: `aptos help` and `aptos account create`
        .usage(
            Style::new()
                .bold()
                .fg_color(Some(Ansi(anstyle::AnsiColor::Yellow))),
        )
        // Most literals like command names and other pieces
        // To test: `aptos help` and `aptos account create`
        .literal(Style::new().fg_color(Some(Ansi(anstyle::AnsiColor::Green))))
        // The word error when an error occurs
        // This is listed as "bright red" to help with red / green colorblindness
        // To test: `aptos account create`
        .error(Style::new().fg_color(Some(Ansi(anstyle::AnsiColor::BrightRed))))
        // Placeholder eg. <ACCOUNT>
        // To test: `aptos account create` or `aptos account create --help`
        .placeholder(Style::new().fg_color(Some(Ansi(anstyle::AnsiColor::Green))))
        // Valid when providing help for missing arguments
        // To test: `aptos account create`
        .valid(Style::new().fg_color(Some(Ansi(anstyle::AnsiColor::Green))))
        // Invalid value during parsing
        // To test: `aptos account create --account not-a-number`
        .invalid(Style::new().fg_color(Some(Ansi(anstyle::AnsiColor::Yellow))))
}

/// Easy way to add CLI completions
pub fn generate_cli_completions<Tool: CommandFactory>(
    tool_name: &str,
    shell: clap_complete::shells::Shell,
    output_file: &Path,
) -> std::io::Result<()> {
    let mut command = Tool::command();
    let mut file = std::fs::File::create(output_file)?;
    clap_complete::generate(shell, &mut command, tool_name, &mut file);
    Ok(())
}
