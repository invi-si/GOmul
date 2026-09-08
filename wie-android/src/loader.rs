use std::fs;
use wie_backend::{Emulator, Options, Platform, extract_zip};
use wie_j2me::J2MEEmulator;
use wie_ktf::KtfEmulator;
use wie_lgt::LgtEmulator;
use wie_skt::SktEmulator;
pub fn load(filename: &str, platform: Box<dyn Platform>) -> anyhow::Result<Box<dyn Emulator>> {
    let options = Options {
        enable_gdbserver: false,
        profile: None,
    };
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let buf = fs::read(filename)?;
    let emulator: Box<dyn Emulator> = if extension == "zip" {
        let files = extract_zip(&buf)?;

        if KtfEmulator::loadable_archive(&files) {
            Box::new(KtfEmulator::from_archive(platform, files, options)?)
        } else if LgtEmulator::loadable_archive(&files) {
            Box::new(LgtEmulator::from_archive(platform, files, options)?)
        } else if SktEmulator::loadable_archive(&files) {
            Box::new(SktEmulator::from_archive(platform, files)?)
        } else {
            anyhow::bail!("Unknown archive format");
        }
    } else if extension == "jad" {
        let jar_filename = filename.replace(".jad", ".jar");
        let jar = fs::read(&jar_filename)?;

        let jar_filename = jar_filename[jar_filename.rfind('/').unwrap_or(0) + 1..].to_owned();

        Box::new(J2MEEmulator::from_jad_jar(platform, buf, jar_filename, jar)?)
    } else if extension == "jar" {
        let filename_without_path = filename[filename.rfind('/').unwrap_or(0) + 1..].to_owned();
        let filename_without_ext = std::path::Path::new(&filename_without_path)
            .file_stem()
            .and_then(|x| x.to_str())
            .unwrap_or(&filename_without_path);

        if KtfEmulator::loadable_jar(&buf) {
            Box::new(KtfEmulator::from_jar(
                platform,
                &filename_without_path,
                buf,
                filename_without_ext,
                filename_without_ext,
                None,
                options,
            )?)
        } else if LgtEmulator::loadable_jar(&buf) {
            Box::new(LgtEmulator::from_jar(
                platform,
                &filename_without_path,
                buf,
                filename_without_ext,
                filename_without_ext,
                None,
                options,
            )?)
        } else if SktEmulator::loadable_jar(&buf) {
            Box::new(SktEmulator::from_jar(platform, &filename_without_path, buf, filename_without_ext, None)?)
        } else {
            Box::new(J2MEEmulator::from_jar(platform, &filename_without_path, buf)?)
        }
    } else {
        anyhow::bail!("Unknown file format");
    };

    Ok(emulator)
}
