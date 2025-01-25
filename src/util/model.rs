use crate::{model, texture};
use std::io::{BufReader, Cursor};

use cfg_if::cfg_if;
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum DDDModel {
    Apple,
    Shark,
}

impl ToString for DDDModel {
    fn to_string(&self) -> String {
        match self {
            DDDModel::Apple => "apple".to_string(),
            DDDModel::Shark => "shark".to_string(),
        }
    }
}

impl DDDModel {
    pub(crate) fn get(&self) -> (String, Vec<u8>) {
        match self {
            DDDModel::Apple => (
                include_str!("../resources/models/apple.obj").parse().unwrap(),
                Vec::from(include_bytes!("../resources/textures/apple.png"))
            ),
            DDDModel::Shark => (
                include_str!("../resources/models/shark.obj").parse().unwrap(),
                Vec::from(include_bytes!("../resources/textures/shark.png"))
            )
        }
    }
}