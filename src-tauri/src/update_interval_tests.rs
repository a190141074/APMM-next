//! 更新设置兼容验收：新值以分钟保存，旧小时值保留真实时长。
use super::*;
use serde_json::json;

#[test]
fn minutes_defaults_and_legacy_hours_keep_their_duration() {
    assert_eq!(Settings::default().update_interval_minutes(), 60);
    for (hours, minutes) in [(1, 60), (24, 1440), (720, 43200), (0, 1440), (721, 1440)] {
        let settings: Settings = serde_json::from_value(json!({
            "updateIntervalHours": hours
        }))
        .unwrap();
        assert_eq!(settings.update_interval_minutes(), minutes);
        let roundtrip: Settings =
            serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap();
        assert_eq!(roundtrip.update_interval_minutes(), minutes);
        assert_eq!(
            crate::dto::WorkspaceSettings::from(&settings).update_interval_minutes,
            minutes
        );
    }
}

#[test]
fn minutes_take_priority_and_validate_range_without_overflow() {
    let settings: Settings = serde_json::from_value(json!({
        "updateIntervalMinutes": 5, "updateIntervalHours": 24
    }))
    .unwrap();
    assert_eq!(settings.update_interval_minutes(), 5);
    assert_eq!(normalized_update_interval_minutes(1), 1);
    assert_eq!(normalized_update_interval_minutes(43200), 43200);
    for raw in [0, 43201, u64::MAX] {
        assert_eq!(normalized_update_interval_minutes(raw), 60);
    }
    let mut updated = settings;
    updated.update_interval_minutes = Some(1);
    updated.update_interval_hours = None;
    let value = serde_json::to_value(&updated).unwrap();
    assert_eq!(value["updateIntervalMinutes"], 1);
    assert!(value.get("updateIntervalHours").is_none());
}
