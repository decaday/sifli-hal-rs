#![doc = include_str!("../README.md")]

pub mod ptab;
pub mod ftab;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::io::Read;
    use std::collections::HashSet;

    /// Test helper function to compare binary files word by word
    /// Returns a set of addresses where differences were found
    fn compare_binary_files(file1: &[u8], file2: &[u8]) -> HashSet<u32> {
        let mut diff_addresses = HashSet::new();
        
        // Ensure files are the same length and word-aligned
        // because _init and _fini generated by gccc, SDK compiled version is longer
        // than sifli-flash-table by 8
        assert_eq!((file1.len() as isize - file2.len() as isize).abs(), 8, "Files length error");
        let len = file1.len().min(file2.len());
        
        // Compare word by word (4 bytes)
        for i in (0..len).step_by(4) {
            let word1 = u32::from_le_bytes(file1[i..i+4].try_into().unwrap());
            let word2 = u32::from_le_bytes(file2[i..i+4].try_into().unwrap());
            
            if word1 != word2 {
                diff_addresses.insert(i as u32);
            }
        }
        
        diff_addresses
    }

    /// Find all test cases in test directory
    fn find_test_cases() -> Vec<PathBuf> {
        let mut test_cases = Vec::new();
        let test_dir = Path::new("test");
        
        for entry in fs::read_dir(test_dir).expect("Failed to read test directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();
            if path.is_dir() {
                let ptab_path = path.join("ptab.json");
                if ptab_path.exists() {
                    test_cases.push(path);
                }
            }
        }
        
        test_cases
    }

    #[test]
    fn test_ptab_ftab_conversion() {
        let test_cases = find_test_cases();
        assert!(!test_cases.is_empty(), "No test cases found");

        for test_case in test_cases {
            println!("Testing case: {:?}", test_case);

            // Read and process PTAB file
            let ptab_path = test_case.join("ptab.json");
            let ptab_contents = fs::read_to_string(&ptab_path)
                .expect("Failed to read PTAB file");
            let ptab = ptab::Ptab::new(&ptab_contents)
                .expect("Failed to parse PTAB JSON");

            // Create and process Ftab
            let mut ftab = ftab::Ftab::new();
            ftab.apply(&ptab);
            let generated_bytes = ftab.to_bytes();

            // Read reference binary
            let reference_path = test_case.join("ftab.bin");
            let mut reference_bytes = Vec::new();
            fs::File::open(&reference_path)
                .expect("Failed to open reference file")
                .read_to_end(&mut reference_bytes)
                .expect("Failed to read reference file");

            // Compare files
            let diff_addresses = compare_binary_files(&generated_bytes, &reference_bytes);
            
            // Filter out expected differences
            let filtered_diffs: HashSet<u32> = diff_addresses
                .into_iter()
                .filter(|&addr| {
                    // ignore hcpu code length
                    addr != 0x1200 && 
                    // ignore bootloader code length
                    addr != 0x1400
                })
                .collect();

            // Assert no unexpected differences
            assert!(
                filtered_diffs.is_empty(),
                "Unexpected differences found at addresses: {:?} in test case {:?}",
                filtered_diffs,
                test_case
            );
        }
    }
}