use structopt::StructOpt;


use std::process::Command;
use std::process::Stdio;
use std::path::Path;
use std::path::PathBuf;
use std::io::Result;

use cargo_metadata::{MetadataCommand, CargoOpt};
use cargo_metadata::Package;
use cargo_metadata::Metadata;
use cargo_metadata::Target;


#[derive(Debug,StructOpt)]
#[structopt(
    about = "Nj Command Line Interface",
    author = "",
    name = "fluvio")]
enum Opt {
    #[structopt(name = "build")]
    Build(BuildOpt)
}

#[derive(Debug,StructOpt)]
struct BuildOpt {

    #[structopt(short = "o", long = "out", default_value = "dist")]
    output: String
}


fn main() {

    let opt = Opt::from_args();

    match opt {
        Opt::Build(opt) => {
            build(opt)
        }
    }
}


// kick off build
fn build(opt: BuildOpt) {

    let mut build_command = Command::new("cargo")
        .arg("build")
        .stdout(Stdio::inherit())
        .spawn()
        .expect("Failed to execute command");

    build_command.wait()
        .expect("failed to wait on child");

    copy_lib(opt.output);

}

fn copy_lib(out: String) {

    let manifest_path = manifest_path();
    let metadata = load_metadata(&manifest_path);
    if let Some(package) = find_current_package(&metadata,&manifest_path) {
        if let Some(target) = find_cdylib(&package) {
            let lib_path = lib_path(&metadata.target_directory,"debug",&target.name);
            copy_cdylib(&lib_path,&out).expect("copy failed");
        } else {
            eprintln!("no cdylib target was founded");
        }
    } else {
        eprintln!("no valid Cargo.toml was founded");
    }
}

fn find_cdylib(package: &Package) -> Option<&Target> {

    for target in &package.targets {
        if target.name == package.name {
            return Some(target)
        }
    }
    None
}


fn find_current_package<'a>(metadata: &'a Metadata,manifest_path: &Path) -> Option<&'a Package> {

    for package in &metadata.packages {
        //println!("package names target: {:#?}",package.name);
        if package.manifest_path == manifest_path {
            return Some(package)
        }
    }

    None

}

fn load_metadata(manifest_path: &Path) -> Metadata {

    MetadataCommand::new()
        .manifest_path(manifest_path)
        .features(CargoOpt::AllFeatures)
        .exec()
        .expect("cargo metadata")
}

fn manifest_path() -> PathBuf {
    let current_path = std::env::current_dir().expect("can't get current directory");
    current_path.join("Cargo.toml")
}

fn lib_path(target: &Path,build_type: &str,target_name: &str) -> PathBuf {

    let file_name = format!("lib{}.dylib",target_name).replace("-","_");
    target.join(target).join(build_type).join(file_name)
}

// where we are outputting
fn output_dir(output: &str) -> Result<PathBuf> {

    let current_path = std::env::current_dir().expect("can't get current directory");
    let output_dir = current_path.join(output);
    // ensure we have directory
    std::fs::create_dir_all(&output_dir)?;

    Ok(output_dir)
}

fn copy_cdylib(lib_path: &Path,out: &str) -> Result<()> {

    let dir = output_dir(out)?;
    let output_path = dir.join("index.node");
    std::fs::copy(lib_path,output_path)?;
    Ok(())
}


