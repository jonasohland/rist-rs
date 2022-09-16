#![allow(unused)]

#[derive(Debug, Clone, Copy)]
pub enum DTLSVersion {
    Version1_0,
    Version1_2,
}

#[derive(Debug, Clone, Copy)]
pub enum Constraint {
    DTLSAllowed,
    DTLSVersion(DTLSVersion),
}
