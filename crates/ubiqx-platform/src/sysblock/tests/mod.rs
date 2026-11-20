//! Tests of the system layer. Every one runs against the fakes of `testing`: an in-memory
//! file system and pretend machines. None touches the real system.

mod cycle;
mod elevate;
mod hardening;
mod helper_args;
mod hosts_write;
mod parsers;
mod policies;
mod resolver;
mod shield;
mod sys_ports;
