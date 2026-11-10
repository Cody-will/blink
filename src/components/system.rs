use sysinfo::{Cpu, Gpu, System, Users, User};

use crate::components::Component;


#[derive(Debug, Clone)]
pub struct Sys {
    pub os: String,
    pub kernel: String,
    pub total_ram: String,
    pub used_ram: String,
    pub cpu: String,
}


impl Sys {
    pub fn new() -> anyhow::Result<Self> {
        let sys = System::new_all()?;

        let os = System::name().unwrap_or_default();
        let kernel = System::kernel_version().unwrap_or_default();

        let total_ram = sys.total_memory() as f64 / 1024.0_f64.powi(3);
        let used_ram = sys.used_memory() as f64 / 1024.0_f64.powi(3);

        let cpu = sys.cpus()
            .first()
            .map(|cpu| cpu.brand())
            .unwrap_or("Unkown CPU");

        Ok(Self { os, kernel, total_ram: format!("{total_ram}"), used_ram: format!("{used_ram}"), cpu: cpu.into() })
    }
}


#[derive(Clone, Debug)]
pub struct Os {
    content: String,
}

impl Component for Os {
    fn content(&self) -> &str {
        &self.content
    } 
}

#[derive(Debug, Clone)]
pub struct Kernel {
    content: String,
}

impl Component for Kernel {
    fn content(&self) -> &str {
        &self.content
    } 
}




