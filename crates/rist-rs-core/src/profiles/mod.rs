#![allow(unused)]

pub mod advanced;
pub mod main;
pub mod simple;

#[derive(Debug, Clone, Copy)]
enum Constraint {
    MainProfileConstraint(main::Constraint),
}
