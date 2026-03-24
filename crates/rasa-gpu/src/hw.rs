//! Hardware profiling via muharrir — detects GPU capabilities and selects
//! an appropriate quality tier for rendering operations.

pub use muharrir::hw::{HardwareProfile, QualityTier};

/// Detect hardware and select the best backend accordingly.
///
/// Returns the detected profile and whether to force CPU fallback.
pub fn detect_and_select() -> (HardwareProfile, bool) {
    let profile = HardwareProfile::detect();
    let force_cpu = !profile.has_gpu || profile.quality == QualityTier::Low;

    tracing::info!(
        "Hardware: {} ({}), quality={}, VRAM={}",
        profile.device_name,
        if profile.has_gpu { "GPU" } else { "CPU" },
        profile.quality,
        profile.gpu_memory_display(),
    );

    (profile, force_cpu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile() {
        let profile = HardwareProfile::default();
        assert_eq!(profile.quality, QualityTier::Medium);
        assert!(!profile.has_gpu);
        assert_eq!(profile.device_name, "Unknown");
    }

    #[test]
    fn quality_tiers_ordered() {
        // Verify we can compare quality tiers
        assert_ne!(QualityTier::Low, QualityTier::High);
        assert_eq!(QualityTier::default(), QualityTier::Medium);
    }

    #[test]
    fn detect_does_not_panic() {
        // In CI, GPU may not be available — just verify no panic
        let (profile, _force_cpu) = detect_and_select();
        // Profile should always have some device name
        assert!(!profile.device_name.is_empty());
    }
}
