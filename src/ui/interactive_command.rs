use std::process::{Command, ExitStatus};

use anyhow::Context;
use cursive::reexports::log::info;
use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;

#[derive(Debug)]
pub(crate) enum InteractiveCommand {
    Exec(Pod, String),
}

impl InteractiveCommand {
    pub(crate) fn run(&self) -> anyhow::Result<ExitStatus> {
        match self {
            InteractiveCommand::Exec(pod, container_name) => {
                let pod_msg_name = format!(
                    "{}/{}/{}",
                    pod.namespace().unwrap_or_default(),
                    pod.name_any(),
                    container_name
                );
                info!("Running exec: {}", pod_msg_name);

                let mut command = Command::new("kubectl");
                command.args([
                    "exec",
                    "-it",
                    pod.name_any().as_str(),
                    "-n",
                    pod.namespace().context("Didnt have a namespace")?.as_str(),
                    "-c",
                    container_name.as_str(),
                    "--",
                    "bash",
                ]);

                info!("Prepared command: {:?}", command);
                let exit_status = command.spawn()?.wait()?;
                Ok(exit_status)
            }
        }
    }
}
