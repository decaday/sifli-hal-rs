use crate::time::Hertz;
use crate::pac::{HPSYS_RCC, HPSYS_AON, PMUC};

pub use crate::pac::hpsys_rcc::vals::{
    SelSys as ClkSysSel,
    SelUsbc as UsbSel,
    SelTick as TickSel,
    SelPeri as ClkPeriSel,
};

/// all clocks:
/// clk_sys, clk_peri, clk_aud_pll
/// hxt48, hrc48
/// clk_dll1, clk_dll2
/// hclk, pclk1, pclk2
/// clk_usb
/// TODO: lxt32, lrc32, lrc10, clk_wdt


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
                PMUC.hxt_cr1().modify(|w| w.set_buf_dll_en(true));
                // Enable DLL1
                HPSYS_RCC.dllcr(0).modify(|w| w.set_en(true));
                // Set DLL1 multiplication factor
                HPSYS_RCC.dllcr(0).modify(|w| w.set_stg(dll1.stg));
                // Enable DLL1 output frequency division by 2
                HPSYS_RCC.dllcr(0).modify(|w| w.set_out_div2_en(dll1.div2));
            } else {
                // Disable DLL1
                HPSYS_RCC.dllcr(0).modify(|w| w.set_en(false));
            }
        }

        if let ConfigOption::Update(dll2) = &self.dll2 {
            if dll2.enable {
                PMUC.hxt_cr1().modify(|w| w.set_buf_dll_en(true));
                // Enable DLL1
                HPSYS_RCC.dllcr(1).modify(|w| w.set_en(true));
                // Set DLL1 multiplication factor
                HPSYS_RCC.dllcr(1).modify(|w| w.set_stg(dll2.stg));
                // Enable DLL1 output frequency division by 2
                HPSYS_RCC.dllcr(1).modify(|w| w.set_out_div2_en(dll2.div2));
            } else {
                // Disable DLL1
                HPSYS_RCC.dllcr(1).modify(|w| w.set_en(false));
            }
        }

        // Configure clock selectors and dividers
        if let ConfigOption::Update(div) = self.hclk_div {
            HPSYS_RCC.cfgr().modify(|w| w.set_hdiv(div));
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

        // Configure system clock selection last
        if let ConfigOption::Update(sel) = self.clk_sys_sel {
            HPSYS_RCC.csr().modify(|w| w.set_sel_sys(sel));
        }
    }
}

/// clk_sys
pub fn get_clk_sys_freq() -> Option<Hertz> {
    match HPSYS_RCC.csr().read().sel_sys() {
        ClkSysSel::Hrc48 => get_hrc48_freq(),
        ClkSysSel::Hxt48 => get_hxt48_freq(),
        ClkSysSel::Dbl96 => todo!(),
        ClkSysSel::Dll1 => get_clk_dll1_freq(),
    }
}

pub fn get_clk_peri_freq() -> Option<Hertz> {
    match HPSYS_RCC.csr().read().sel_peri() {
        ClkPeriSel::Hxt48 => get_hxt48_freq(),
        ClkPeriSel::Hrc48 => get_hrc48_freq(),
    }
}
pub fn get_clk_peri_div2_freq() -> Option<Hertz> {
    match HPSYS_RCC.csr().read().sel_peri() {
        ClkPeriSel::Hxt48 => get_hxt48_freq().map(|f| f / 2u8),
        ClkPeriSel::Hrc48 => get_hrc48_freq().map(|f| f / 2u8),
    }
}

pub fn get_hclk_freq() -> Option<Hertz> {
    let clk_sys = get_clk_sys_freq()?;
    Some(clk_sys / HPSYS_RCC.cfgr().read().hdiv())
}

pub fn get_pclk1_freq() -> Option<Hertz> {
    let hclk = get_hclk_freq()?;
    Some(hclk / (1 << HPSYS_RCC.cfgr().read().pdiv1()) as u32)
}

pub fn get_pclk2_freq() -> Option<Hertz> {
    let hclk = get_hclk_freq()?;
    Some(hclk / (1 << HPSYS_RCC.cfgr().read().pdiv2()) as u32)
}

pub fn get_hxt48_freq() -> Option<Hertz> {
    if HPSYS_AON.acr().read().hxt48_rdy() {
        Some(Hertz(48_000_000))
    } else {
        None
    }
}

pub fn get_hrc48_freq() -> Option<Hertz> {
    if HPSYS_AON.acr().read().hrc48_rdy() {
        Some(Hertz(48_000_000))
    } else {
        None
    }
}

pub fn get_clk_dll1_freq() -> Option<Hertz> {
    let dllcr = HPSYS_RCC.dllcr(0).read();
    if dllcr.en() {
        Some(Hertz(24_000_000 * (dllcr.stg() + 1) as u32 / (dllcr.out_div2_en() as u32 + 1)))
    } else {
        None
    }
}

pub fn get_clk_dll2_freq() -> Option<Hertz> {
    let dllcr = HPSYS_RCC.dllcr(1).read();
    if dllcr.en() {
        Some(Hertz(24_000_000 * (dllcr.stg() + 1) as u32 / (dllcr.out_div2_en() as u32 + 1)))
    } else {
        None
    }
}

pub fn get_clk_usb_freq() -> Option<Hertz> {
    match HPSYS_RCC.csr().read().sel_usbc() {
        UsbSel::ClkSys => get_clk_sys_freq(),
        UsbSel::Dll2 => get_clk_dll2_freq(),
    }
}

pub fn get_clk_aud_pll_freq() -> Option<Hertz> {
    Some(Hertz(49_152_000))
}

pub fn test_print_clocks() {
    defmt::info!("Clock frequencies:");
    
    let clocks = [
        ("clk_sys", get_clk_sys_freq()),
        ("clk_peri", get_clk_peri_freq()),
        ("clk_peri_div2", get_clk_peri_div2_freq()),
        ("hclk", get_hclk_freq()),
        ("pclk1", get_pclk1_freq()),
        ("pclk2", get_pclk2_freq()),
        ("hxt48", get_hxt48_freq()),
        ("hrc48", get_hrc48_freq()),
        ("clk_dll1", get_clk_dll1_freq()),
        ("clk_dll2", get_clk_dll2_freq()),
        ("clk_usb", get_clk_usb_freq()),
        ("clk_aud_pll", get_clk_aud_pll_freq()),
    ];

    for (name, freq) in clocks {
        if let Some(f) = freq {
            let freq_khz = f.0 / 1_000;
            defmt::info!("{}: {}.{:03} MHz", 
                name,
                freq_khz / 1_000,
                freq_khz % 1_000
            );
        } else {
            defmt::info!("{}: disabled", name);
        }
    }
}
