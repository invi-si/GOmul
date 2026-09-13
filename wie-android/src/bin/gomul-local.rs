fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let archive = args.next().ok_or_else(|| anyhow::anyhow!("Missing archive"))?;
    let save = args.next().ok_or_else(|| anyhow::anyhow!("Missing save directory"))?;
    wie_android::local_web::serve(archive, save)
}
