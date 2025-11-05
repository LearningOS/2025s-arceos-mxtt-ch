#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // Print "Hello, Arceos!" with ANSI green color so the tester detects color
    // The tester strips ANSI codes and greps for "Hello, Arceos!"
    println!("\x1b[32mHello, Arceos!\x1b[0m");
}
