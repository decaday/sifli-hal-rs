use crate::pac::HPSYS_CFG;

// Constants for DVFS mode limits
pub const HPSYS_DVFS_MODE_D0_LIMIT: u32 = 24;
pub const HPSYS_DVFS_MODE_D1_LIMIT: u32 = 48;
pub const HPSYS_DVFS_MODE_S0_LIMIT: u32 = 144;
pub const HPSYS_DVFS_MODE_S1_LIMIT: u32 = 240;

pub const HPSYS_DVFS_CONFIG: [HpsysDvfsConfig; 4] = [
    // LDO: 0.9V, BUCK: 1.0V
    HpsysDvfsConfig { ldo_offset: -5, ldo: 0x6, buck: 0x9, ulpmcr: 0x00100330 },
    // LDO: 1.0V, BUCK: 1.1V
    HpsysDvfsConfig { ldo_offset: -3, ldo: 0x8, buck: 0xA, ulpmcr: 0x00110331 },
    // LDO: 1.1V, BUCK: 1.25V
    HpsysDvfsConfig { ldo_offset:  0, ldo: 0xB, buck: 0xD, ulpmcr: 0x00130213 },
    // LDO: 1.2V, BUCK: 1.35V
    HpsysDvfsConfig { ldo_offset:  2, ldo: 0xD, buck: 0xF, ulpmcr: 0x00130213 },
];

pub const HPSYS_DLL2_LIMIT: [u32; 4] = [
    0,           // D0 Mode
    0,           // D1 Mode
    288_000_000, // S0 Mode
    288_000_000, // S1 Mode
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpsysDvfsMode {
    D0 = 0,
    D1 = 1,
    S0 = 2,
    S1 = 3,
}

pub fn is_hpsys_dvfs_mode_s() -> bool {
    HPSYS_CFG.syscr().read().ldo_vsel()
}

impl HpsysDvfsMode {
    pub fn from_frequency(freq_mhz: u32) -> Result<Self, &'static str> {
        match freq_mhz {
            0..=HPSYS_DVFS_MODE_D0_LIMIT => Ok(HpsysDvfsMode::D0),
            25..=HPSYS_DVFS_MODE_D1_LIMIT => Ok(HpsysDvfsMode::D1),
            49..=HPSYS_DVFS_MODE_S0_LIMIT => Ok(HpsysDvfsMode::S0),
            145..=HPSYS_DVFS_MODE_S1_LIMIT => Ok(HpsysDvfsMode::S1),
            _ => Err("Frequency out of valid range"),
        }
    }

    pub fn get_dll2_limit(self) -> u32 {
        HPSYS_DLL2_LIMIT[self as usize]
    }

    pub fn get_config(self) -> HpsysDvfsConfig {
        HPSYS_DVFS_CONFIG[self as usize]
    }

    pub fn get_frequency_limit(self) -> u32 {
        match self {
            HpsysDvfsMode::D0 => HPSYS_DVFS_MODE_D0_LIMIT,
            HpsysDvfsMode::D1 => HPSYS_DVFS_MODE_D1_LIMIT,
            HpsysDvfsMode::S0 => HPSYS_DVFS_MODE_S0_LIMIT,
            HpsysDvfsMode::S1 => HPSYS_DVFS_MODE_S1_LIMIT,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HpsysDvfsConfig {
    pub ldo_offset: i8,
    pub ldo: u8,
    pub buck: u8,
    pub ulpmcr: u32,
}


