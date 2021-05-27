use std::cmp::Ordering;

use crate::cri::runtime::Spec;
use derive_builder::Builder;
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Builder, Getters, Serialize, Deserialize)]
#[builder(default, pattern = "owned", setter(into, strip_option))]
/// A general OCI container implementation.
pub struct OCIContainer {
	#[get = "pub"]
	/// Unique identifier of the container.
	id: String,

	#[get = "pub"]
	bundle: String,

	#[get = "pub"]
	/// OCI Runtime Specification of the container.
	spec: Spec,
}

impl OCIContainer {
	pub fn new(bundle: String, id: String) -> Self {
		let mut config = std::path::PathBuf::from(bundle.clone());
		config.push("config.json");
		let can_path =
			std::fs::canonicalize(bundle.clone()).expect("Unable to determine absolute path");

		Self {
			id: id,
			bundle: can_path.to_str().unwrap().to_string(),
			spec: Spec::from(&config).expect("Unable to load config file"),
		}
	}
}

impl Eq for OCIContainer {}

impl Ord for OCIContainer {
	fn cmp(&self, other: &Self) -> Ordering {
		self.id.cmp(&other.id)
	}
}

impl PartialOrd for OCIContainer {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for OCIContainer {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}
