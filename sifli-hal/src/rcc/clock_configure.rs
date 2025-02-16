use crate::pac::{HPSYS_RCC, HPSYS_AON, HPSYS_CFG, PMUC};
use crate::time::Hertz;

use super::{ClkSysSel, ClkPeriSel, UsbSel, TickSel};

/// Represents a configuration value that can either be updated with a new value
/// or kept unchanged from its previous state.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigOption<T> {
    /// Update the configuration with a new value
    Update(T),
    /// Keep the existing configuration value unchanged
    Keep,
}

impl<T> ConfigOption<T> {
    /// Creates a new ConfigOption that will update to the given value
    pub fn new(value: T) -> Self {
        ConfigOption::Update(value)
    }

    /// Creates a new ConfigOption that will keep the existing value
    pub fn keep() -> Self {
        ConfigOption::Keep
    }

    /// Returns true if this ConfigOption is set to update with a new value
    pub fn is_update(&self) -> bool {
        matches!(self, ConfigOption::Update(_))
    }

    /// Returns true if this ConfigOption is set to keep the existing value
    pub fn is_keep(&self) -> bool {
        matches!(self, ConfigOption::Keep)
    }

    /// Applies this ConfigOption to an existing value, either updating it or keeping it unchanged
    pub fn apply(self, current: T) -> T {
        match self {
            ConfigOption::Update(new_value) => new_value,
            ConfigOption::Keep => current,
        }
    }
}

pub struct Config {
    /// Enable the 48MHz external crystal oscillator
    pub hxt48_enable: ConfigOption<bool>,
    /// Enable the 48MHz internal RC oscillator
    pub hrc48_enable: ConfigOption<bool>,
    /// Configuration for DLL1
    pub dll1: ConfigOption<DllConfig>,
    /// Configuration for DLL2 
    /// Note: Bootloader typically configures this to 288MHz for PSRAM and external flash
    pub dll2: ConfigOption<DllConfig>,
    /// Select the clock source for system clock (clk_sys)
    pub clk_sys_sel: ConfigOption<ClkSysSel>,
    /// HCLK divider: HCLK = CLK_SYS / hclk_div
    /// Valid range: 0 to 255
    pub hclk_div: ConfigOption<u8>,
    /// PCLK1 divider: PCLK1 = HCLK / 2^pclk1_div
    /// Valid range: 0 to 7
    pub pclk1_div: ConfigOption<u8>,
    /// PCLK2 divider: PCLK2 = HCLK / 2^pclk2_div
    /// Valid range: 0 to 7
    pub pclk2_div: ConfigOption<u8>,
    
    /// USB clock configuration
    pub usb: ConfigOption<UsbConfig>,
    /// Tick clock configuration
    pub tick: ConfigOption<TickConfig>,
    /// Select the clock source for peripheral clock
    pub clk_peri_sel: ConfigOption<ClkPeriSel>,
}

pub struct DllConfig {
    /// Enable/disable the DLL
    pub enable: bool,
    /// DLL multiplication factor
    /// Output frequency = (stg + 1) × 24MHz
    /// Valid range: 0 to 15
    pub stg: u8,
    /// Enable output frequency division by 2
    pub div2: bool,
}

pub struct UsbConfig {
    /// Select the clock source for USB
    pub sel: UsbSel,
    /// USB clock divider: USB_CLK = CLK_SYS / div
    /// Valid range: 0 to 7
    pub div: u8,
}

pub struct TickConfig {
    /// Select the clock source for system tick
    pub sel: TickSel,
    /// System tick divider
    /// Valid range: 0 to 63
    pub div: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hxt48_enable: ConfigOption::new(true),
            hrc48_enable: ConfigOption::new(false),
            dll1: ConfigOption::new(DllConfig { enable: true, stg: 5, div2: false }),
            dll2: ConfigOption::keep(),
            clk_sys_sel: ConfigOption::new(ClkSysSel::Dll1),
            hclk_div: ConfigOption::new(0),
            pclk1_div: ConfigOption::new(0),
            pclk2_div: ConfigOption::new(0),
            usb: ConfigOption::new(UsbConfig { sel: UsbSel::ClkSys, div: 0 }),
            tick: ConfigOption::new(TickConfig { sel: TickSel::ClkRtc, div: 0 }),
            clk_peri_sel: ConfigOption::new(ClkPeriSel::Hxt48),
        }
    }
}

impl Config {
    pub fn new_keep() -> Self {
        Self {
            hxt48_enable: ConfigOption::keep(),
            hrc48_enable: ConfigOption::keep(),
            dll1: ConfigOption::keep(),
            dll2: ConfigOption::keep(),
            clk_sys_sel: ConfigOption::keep(),
            hclk_div: ConfigOption::keep(),
            pclk1_div: ConfigOption::keep(),
            pclk2_div: ConfigOption::keep(),
            usb: ConfigOption::keep(),
            tick: ConfigOption::keep(),
            clk_peri_sel: ConfigOption::keep(),
        }
    }

    /// Apply the RCC clock configuration to the hardware registers
    /// 
    /// Safety
    /// This function is typically called by sifli_hal::init() (configured 
    /// in sifli_hal::Config.rcc), but can also be called independently as 
    /// long as it does not interfere with the clocks of already initialized 
    /// peripherals.
    /// In the Bootloader, FLASH and PSRAM have already been initialized. 
    /// You must ensure that their clocks are not broken.
    /// If configuring the clock after calling sifli_hal::init(), make sure 
    /// not to break the clock of Timer used as the time driver.
    pub unsafe fn apply(&self) {
        // Configure oscillators
        if let ConfigOption::Update(enable) = self.hxt48_enable {
            HPSYS_AON.acr().modify(|w| w.set_hxt48_req(enable));
            while HPSYS_AON.acr().read().hxt48_rdy() != enable {}
        }

        if let ConfigOption::Update(enable) = self.hrc48_enable {
            HPSYS_AON.acr().modify(|w| w.set_hrc48_req(enable));
            while HPSYS_AON.acr().read().hrc48_rdy() != enable {}
        }

        // Configure DLLs
        if let ConfigOption::Update(dll1) = &self.dll1 {
            if dll1.enable {
                rcc_assert!(max::DLL.contains(
                    &self.get_final_dll1_freq().unwrap()
                ));
                
                PMUC.hxt_cr1().modify(|w| w.set_buf_dll_en(true));

                HPSYS_CFG.cau2_cr().modify(|w| {
                    if !w.hpbg_en() { // SDK does this check, but it's not clear why
                        w.set_hpbg_en(true);
                    }
                    if !w.hpbg_vddpsw_en() {
                        w.set_hpbg_vddpsw_en(true);
                    }
                });
                // Enable DLL1
                HPSYS_RCC.dllcr(0).modify(|w| {
                    w.set_en(true);
                    w.set_stg(dll1.stg);
                    w.set_out_div2_en(dll1.div2);
                });
                // SDK: wait for DLL ready, 5us at least
                crate::cortex_m_blocking_delay_us(10);
                while !HPSYS_RCC.dllcr(0).read().ready() {}
            } else {
                // Disable DLL1
                HPSYS_RCC.dllcr(0).modify(|w| w.set_en(false));
            }
        }

        if let ConfigOption::Update(dll2) = &self.dll2 {
            if dll2.enable {
                rcc_assert!(max::DLL.contains(
                    &self.get_final_dll2_freq().unwrap()
                ));

                PMUC.hxt_cr1().modify(|w| w.set_buf_dll_en(true));

                HPSYS_CFG.cau2_cr().modify(|w| {
                    if !w.hpbg_en() { // SDK does this check, but it's not clear why
                        w.set_hpbg_en(true);
                    }
                    if !w.hpbg_vddpsw_en() {
                        w.set_hpbg_vddpsw_en(true);
                    }
                });

                // Enable DLL2
                HPSYS_RCC.dllcr(1).modify(|w| {
                    w.set_en(true);
                    w.set_stg(dll2.stg);
                    w.set_out_div2_en(dll2.div2);
                });
                // SDK: wait for DLL ready, 5us at least
                crate::cortex_m_blocking_delay_us(10);
                while !HPSYS_RCC.dllcr(1).read().ready() {}
            } else {
                // Disable DLL1
                HPSYS_RCC.dllcr(1).modify(|w| w.set_en(false));
            }
        }

        if let ConfigOption::Update(div) = self.pclk1_div {
            HPSYS_RCC.cfgr().modify(|w| w.set_pdiv1(div));
        }
        if let ConfigOption::Update(div) = self.pclk2_div {
            HPSYS_RCC.cfgr().modify(|w| w.set_pdiv2(div));
        }

        // Configure USB clock
        if let ConfigOption::Update(usb_cfg) = &self.usb {
            HPSYS_RCC.csr().modify(|w| w.set_sel_usbc(usb_cfg.sel));
            HPSYS_RCC.usbcr().modify(|w| w.set_div(usb_cfg.div));
        }

        // Configure tick clock
        if let ConfigOption::Update(tick_cfg) = &self.tick {
            HPSYS_RCC.csr().modify(|w| w.set_sel_tick(tick_cfg.sel));
            HPSYS_RCC.cfgr().modify(|w| w.set_tickdiv(tick_cfg.div));
        }

        // Configure peripheral clock
        if let ConfigOption::Update(sel) = self.clk_peri_sel {
            HPSYS_RCC.csr().modify(|w| w.set_sel_peri(sel));
        }

        let hclk_freq = self.get_final_hclk_freq();
        
        // Configure system clock selection last
        if let ConfigOption::Update(sel) = self.clk_sys_sel {
            match sel {
                ClkSysSel::Hrc48 => if self.get_final_hrc48_enable() {
                    panic!("clk_sys_sel is Hrc48, but hrc48 is disabled")
                },
                ClkSysSel::Hxt48 => if self.get_final_hxt48_enable() {
                    panic!("clk_sys_sel is Hxt48, but hxt48 is disabled")
                },
                ClkSysSel::Dbl96 => todo!(),
                ClkSysSel::Dll1 => if self.get_final_dll1_enable() {
                    panic!("clk_sys_sel is dll1, but dll1 is disabled")
                },
            }
            HPSYS_RCC.csr().modify(|w| w.set_sel_sys(sel));
        }

        // Configure clock selectors and dividers
        if let ConfigOption::Update(div) = self.hclk_div {
            HPSYS_RCC.cfgr().modify(|w| w.set_hdiv(div));
        }
    }

    fn get_final_dll1_freq(&self) -> Option<Hertz> {
        if let ConfigOption::Update(dll1) = &self.dll1 {
            if dll1.enable {
                Some(Hertz((dll1.stg + 1) as u32 * 24_000_000 / (dll1.div2 as u32 + 1)))
            } else {
                None
            }
        } else {
            super::get_clk_dll1_freq()
        }
    }

    fn get_final_dll2_freq(&self) -> Option<Hertz> {
        if let ConfigOption::Update(dll2) = &self.dll2 {
            if dll2.enable {
                Some(Hertz((dll2.stg + 1) as u32 * 24_000_000 / (dll2.div2 as u32 + 1)))
            } else {
                None
            }
        } else {
            super::get_clk_dll2_freq()
        }
    }

    fn get_final_clk_sys_freq(&self) -> Option<Hertz> {
        match self.clk_sys_sel {
            ConfigOption::Update(ClkSysSel::Hxt48) => Some(Hertz(48_000_000)),
            ConfigOption::Update(ClkSysSel::Hrc48) => Some(Hertz(48_000_000)),
            ConfigOption::Update(ClkSysSel::Dll1) => self.get_final_dll1_freq(),
            ConfigOption::Update(ClkSysSel::Dbl96) => todo!(),
            ConfigOption::Keep => None,
        }
    }

    fn get_final_hclk_freq(&self) -> Option<Hertz> {
        if self.hclk_is_keep() {
            self.get_final_hclk_freq()
        } else {
            match self.hclk_div {
                ConfigOption::Update(div) => {
                    let clk_sys = self.get_final_clk_sys_freq()?;
                    Some(clk_sys / div as u32)
                },
                ConfigOption::Keep => unreachable!(),
            }
        }
    }

    fn hclk_is_keep(&self) -> bool {
        if self.hclk_div.is_update() && self.clk_sys_sel.is_update() {
            return true
        }

        let is_keep = match super::get_clk_sys_source() {
            ClkSysSel::Hrc48 => false,
            ClkSysSel::Hxt48 => false,
            ClkSysSel::Dbl96 => todo!(),
            ClkSysSel::Dll1 => self.dll1.is_update()
        };
        if is_keep { return true };

        false
    }

    fn get_final_dll1_enable(&self) -> bool {
        if let ConfigOption::Update(dll1) = &self.dll1 {
            if dll1.enable {
                true
            } else {
                false
            }
        } else {
            super::get_clk_dll1_freq().is_some()
        }
    }

    fn get_final_hxt48_enable(&self) -> bool {
        if let ConfigOption::Update(enable) = self.hxt48_enable {
            enable
        } else {
            super::get_hxt48_freq().is_some()
        }
    }

    fn get_final_hrc48_enable(&self) -> bool {
        if let ConfigOption::Update(enable) = self.hrc48_enable {
            enable
        } else {
            super::get_hrc48_freq().is_some()
        }
    }
}

#[cfg(feature = "sf32lb52x")]
mod max {
    use core::ops::RangeInclusive;
    use crate::time::Hertz;

    pub(crate) const DLL: RangeInclusive<Hertz> = Hertz(24_000_000)..=Hertz(384_000_000);
}
