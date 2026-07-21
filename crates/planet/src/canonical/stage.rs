/// The seven fixed causal stages of the full planet arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    StarDiskSystem,
    AssemblyComposition,
    OrbitalSecularMoons,
    YoungThermalMaterials,
    GeodynamicsDeepTime,
    LoadFlexure,
    Snapshot,
}

impl Stage {
    pub const ALL: [Self; 7] = [
        Self::StarDiskSystem,
        Self::AssemblyComposition,
        Self::OrbitalSecularMoons,
        Self::YoungThermalMaterials,
        Self::GeodynamicsDeepTime,
        Self::LoadFlexure,
        Self::Snapshot,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::StarDiskSystem => "star_disk_system",
            Self::AssemblyComposition => "assembly_composition",
            Self::OrbitalSecularMoons => "orbital_secular_moons",
            Self::YoungThermalMaterials => "young_thermal_materials",
            Self::GeodynamicsDeepTime => "geodynamics_deep_time",
            Self::LoadFlexure => "load_flexure",
            Self::Snapshot => "snapshot",
        }
    }
}
