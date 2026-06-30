// SPDX-License-Identifier: MIT
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "atrium.toml".to_string());
    atrium_core::Atrium::run(&config_path)
}
