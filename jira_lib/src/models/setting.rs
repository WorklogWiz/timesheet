use serde::{Deserialize, Serialize};

/// Represents the global Jira settings
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[allow(clippy::struct_excessive_bools)]
pub struct GlobalSettings {
    votingEnabled: bool,
    watchingEnabled: bool,
    unassignedIssuesAllowed: bool,
    subTasksEnabled: bool,
    issueLinkingEnabled: bool,
    timeTrackingEnabled: bool,
    attachmentsEnabled: bool,
    pub(crate) timeTrackingConfiguration: TimeTrackingConfiguration,
}

/// Represents the time tracking configuration settings retrieved from Jira
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TimeTrackingConfiguration {
    /// Holds the number of work hours per day, typically 7.5 in Norway
    pub workingHoursPerDay: f32,
    /// Number of work days per week, typically 5.0
    pub workingDaysPerWeek: f32,
    /// What time format is used
    pub timeFormat: String,
    /// What is the default unit
    pub defaultUnit: String,
}
