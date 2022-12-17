#![no_std]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

mod error;

use ckb_std::default_alloc;

ckb_std::default_alloc!();
ckb_std::entry!(main);

fn main() -> i8 {
	0	
}

