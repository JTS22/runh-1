use nix::mount::MntFlags;
use nix::sched::CloneFlags;
use oci_spec::runtime::LinuxNamespaceType;

use crate::container::OCIContainer;
use crate::kill;
use crate::network;
use crate::state;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::os::unix::prelude::AsRawFd;
use std::path::PathBuf;

fn reset_network_namespace(container_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	let network_file_path = container_dir.join("hermit_network.json");
	if network_file_path.exists() {
		let network_file = std::fs::File::open(network_file_path)?;
		let buf_reader = BufReader::new(network_file);
		let network_config: network::HermitNetworkConfig = serde_json::from_reader(buf_reader)?;
		let namespace_file = File::open(PathBuf::from(network_config.network_namespace.as_ref().unwrap()))?;
		nix::sched::setns(namespace_file.as_raw_fd(), CloneFlags::CLONE_NEWNET)?;

		network::undo_tap_creation(&network_config)?;
	}
	Ok(())
}

pub fn delete_container(mut project_dir: PathBuf, id: Option<&str>, force: bool) {
	let container_state = state::get_container_state(project_dir.clone(), id.unwrap());
	if container_state.status != "stopped" {
		if !force {
			panic!("Tried to delete a container that is not stopped!");
		} else {
			if container_state.status != "creating" {
				warn!("Container is still running. Force-deleting...");
				kill::kill_container(project_dir.clone(), id, Some("SIGKILL"), false);
			} else {
				warn!("Container has not finished creation. Force-deleting...")
			}
		}
	}

	let mut delete_file = false;
	project_dir.push(id.unwrap());
	// path to the container specification
	let container_dir = project_dir.clone();
	project_dir.push("container.json");

	if let Ok(mut file) = fs::OpenOptions::new()
		.read(true)
		.write(false)
		.open(project_dir)
	{
		let mut contents = String::new();
		file.read_to_string(&mut contents)
			.expect("Unable to read container specification");

		if let Ok(_container) = serde_json::from_str::<OCIContainer>(&contents) {
			delete_file = true;
		}
	} else {
		println!("Container `{}` doesn't exists", id.unwrap());
	}

	if delete_file {
		let rootfs_overlay_dir = container_dir.join("rootfs/merged");
		if rootfs_overlay_dir.exists() {
			nix::mount::umount2(&rootfs_overlay_dir, MntFlags::MNT_DETACH)
				.expect(format!("Could not unmount overlay at {:?}", rootfs_overlay_dir).as_str());
		}

		match reset_network_namespace(&container_dir) {
			Ok(_) => {}
			Err(err) => warn!("Failed to reset network namespace! Error: {}", err),
		}

		// delete all temporary files
		fs::remove_dir_all(container_dir).expect("Unable to delete container");
	}

	//Additionally to deleting all the files, we should also delete all remaining processes spawned by the container init process.
	//However, without cgroup support there is currently no real way to do this as we do not know when (and how) the init process will be killed
}
