//! System dựng sẵn. Mỗi feature gameplay mới (bom, lửa, item, nhốt…) là một module ở đây,
//! gắn feature ID trong catalog, và được mode đưa vào pipeline.

mod movement;

pub use movement::MOVEMENT;
