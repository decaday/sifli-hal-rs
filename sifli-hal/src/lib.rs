#![no_std]
#![doc = include_str!("../README.md")]

#[cfg(feature = "set-msplim")]
use core::arch::global_asm;

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod rcc;
pub mod gpio;
pub mod timer;
#[cfg(feature = "_time-driver")]
pub mod time_driver;

// Reexports
pub use embassy_hal_internal::{into_ref, Peripheral, PeripheralRef};
#[cfg(feature = "unstable-pac")]
pub use sifli_pac as pac;
#[cfg(not(feature = "unstable-pac"))]
pub(crate) use sifli_pac as pac;

/// HAL configuration for RP.
pub mod config {
    // use crate::clocks::ClockConfig;

    /// HAL configuration passed when initializing.
    #[non_exhaustive]
    pub struct Config {
        // /// Clock configuration.
        // pub clocks: ClockConfig,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                // clocks: ClockConfig::crystal(12_000_000),
            }
        }
    }

    impl Config {
        // /// Create a new configuration with the provided clock config.
        // pub fn new(clocks: ClockConfig) -> Self {
        //     Self { clocks }
        // }

        pub fn new() -> Self {
            Self {}
        }
    }
}

/// Initialize the `sifli-hal` with the provided configuration.
///
/// This returns the peripheral singletons that can be used for creating drivers.
///
/// This should only be called once at startup, otherwise it panics.
pub fn init(_config: config::Config) -> Peripherals {
    system_init();

    // Do this first, so that it panics if user is calling `init` a second time
    // before doing anything important.
    let p = Peripherals::take();

    unsafe {
        // clocks::init(config.clocks);
        #[cfg(feature = "_time-driver")]
        time_driver::init();
        
        gpio::init();

        // dma::init();
    }
    p
}

fn system_init() {
    unsafe {
        let mut cp = cortex_m::Peripherals::steal();

        // enable CP0/CP1/CP2 Full Access
        cp.SCB.cpacr.modify(|r| {
            r | (0b111111)
        });

        // Enable Cache
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);
    }
}

pub(crate) mod _generated {
    #![allow(dead_code)]
    #![allow(unused_imports)]
    #![allow(non_snake_case)]
    #![allow(missing_docs)]

    include!(concat!(env!("OUT_DIR"), "/_generated.rs"));
}

pub use _generated::interrupt;
pub use _generated::{peripherals, Peripherals};

/// Performs a busy-wait delay for a specified number of microseconds.
#[allow(unused)]
pub(crate) fn blocking_delay_us(us: u32) {
    #[cfg(feature = "time")]
    embassy_time::block_for(embassy_time::Duration::from_micros(us as u64));
    #[cfg(not(feature = "time"))]
    {
        todo!();
        // let freq = unsafe { crate::rcc::get_freqs() }.sys.to_hertz().unwrap().0 as u64;
        // let us = us as u64;
        // let cycles = freq * us / 1_000_000;
        // cortex_m::asm::delay(cycles as u32);
    }
}

/// Macro to bind interrupts to handlers.
///
/// This defines the right interrupt handlers, and creates a unit struct (like `struct Irqs;`)
/// and implements the right [`Binding`]s for it. You can pass this struct to drivers to
/// prove at compile-time that the right interrupts have been bound.
///
/// Example of how to bind one interrupt:
///
/// ```rust,ignore
/// use sifli_hal::{bind_interrupts, usb, peripherals};
///
/// bind_interrupts!(struct Irqs {
///     USBCTRL_IRQ => usb::InterruptHandler<peripherals::USB>;
/// });
/// ```
///
// developer note: this macro can't be in `embassy-hal-internal` due to the use of `$crate`.
#[macro_export]
macro_rules! bind_interrupts {
    ($vis:vis struct $name:ident {
        $(
            $(#[cfg($cond_irq:meta)])?
            $irq:ident => $(
                $(#[cfg($cond_handler:meta)])?
                $handler:ty
            ),*;
        )*
    }) => {
        #[derive(Copy, Clone)]
        $vis struct $name;

        $(
            #[allow(non_snake_case)]
            #[no_mangle]
            $(#[cfg($cond_irq)])?
            unsafe extern "C" fn $irq() {
                $(
                    $(#[cfg($cond_handler)])?
                    <$handler as $crate::interrupt::typelevel::Handler<$crate::interrupt::typelevel::$irq>>::on_interrupt();

                )*
            }

            $(#[cfg($cond_irq)])?
            $crate::bind_interrupts!(@inner
                $(
                    $(#[cfg($cond_handler)])?
                    unsafe impl $crate::interrupt::typelevel::Binding<$crate::interrupt::typelevel::$irq, $handler> for $name {}
                )*
            );
        )*
    };
    (@inner $($t:tt)*) => {
        $($t)*
    }
}

// Check README.md for details
#[cfg(feature = "set-msplim")]
global_asm!(
    ".section .text._pre_init",
    ".global __pre_init",
    ".type __pre_init, %function",
    ".thumb_func",
    "__pre_init:",
    "    ldr r0, =_stack_end",
    "    msr MSPLIM, r0",
    "    bx lr",
    ".size __pre_init, . - __pre_init"
);