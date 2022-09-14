#![allow(unused)]

pub mod simple;
pub mod main;
pub mod advanced;

#[derive(Debug, Clone, Copy)]
enum Constraint {
    MainProfileConstraint(main::Constraint)
}