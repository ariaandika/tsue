



fn main() -> anyhow::Result<()> {
    vice::runtime::listen_blocking("0.0.0.0:3000")
}
