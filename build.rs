use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
   let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();	
	  if !Path::new("./ext4.img").exists() {
         Command::new("dd")
            .args(&["if=/dev/zero", "of=ext4.img", "bs=4k", "count=2048"])
            .current_dir(&Path::new(&project_dir))
            .status()
            .unwrap();
        Command::new("mkfs.ext4")
            .args(&["ext4.img"])
            .current_dir(&Path::new(&project_dir))
            .status()
            .unwrap();
    }
}

