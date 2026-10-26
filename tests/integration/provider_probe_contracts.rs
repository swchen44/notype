//! Headless production-helper contracts; independent of Tauri or live providers.
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_probe/mod.rs"]
mod probe;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use notype_core::{SessionOptions, SessionService};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
use workflow::TextConfiguration;
mod provider_probe_cli;
mod provider_probe_evidence;
#[path = "../unittest/provider_probe_resources.rs"]
mod provider_probe_resources;
mod provider_probe_runtime;
mod provider_probe_smoke;
mod provider_probe_transport;
