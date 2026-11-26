//! What the player is doing, for Discord Rich Presence (`horizon_client::discord_presence`).
use super::*;
use horizon_client::discord_presence::{
    self as presence, Dimension, Play, PresenceDetails, ReplayView, Status,
};

impl Game<'_> {
    /// Each frame: only a change reaches the presence thread.
    pub(super) fn update_discord_presence(&self) {
        // One Discord status per computer: P1's, not a splitscreen player's.
        if self.options.split_guest {
            return;
        }
        let (status, details) = self.discord_status_and_details();
        presence::set_detailed(status, details);
    }

    fn discord_status_and_details(&self) -> (Status, PresenceDetails) {
        if !self.settings.extras.discord_presence {
            return (Status::Off, PresenceDetails::default());
        }
        let shaders_active = !self.settings.horizon.shader_pack.is_empty();
        let screen_recording = horizon_client::recorder::status().phase.active();
        let music_now_playing = horizon_client::internet_music::now_playing_for_discord();
        if let Some(replay) = &self.replay {
            let view = match replay.camera_mode {
                horizon_client::replay::CameraMode::FreeCam => ReplayView::FreeCam,
                horizon_client::replay::CameraMode::Spectate(_) => ReplayView::Spectating,
                horizon_client::replay::CameraMode::PathPreview => ReplayView::PathPreview,
                horizon_client::replay::CameraMode::RenderPath => ReplayView::Rendering,
            };
            return (
                Status::Replay {
                    view,
                    paused: !replay.playing,
                },
                PresenceDetails {
                    replay_name: Some(replay.meta.server_name.clone()),
                    shaders_active,
                    screen_recording,
                    music_now_playing,
                    ..Default::default()
                },
            );
        }
        if self.title.is_some() {
            return (
                Status::Menus,
                PresenceDetails {
                    music_now_playing,
                    ..Default::default()
                },
            );
        }
        let play = if let Some(host) = &self.lan {
            Play::Hosting {
                players: 1 + host.hub.guests.len(),
            }
        } else if let Some(server) = &self.integrated {
            match (server.published(), &self.remote) {
                (Some(_), Some(remote)) => Play::Hosting {
                    players: remote.player_count(),
                },
                _ => Play::Singleplayer,
            }
        } else if let Some(remote) = &self.remote {
            // How many the server lists; its address and name are never sent.
            Play::Multiplayer {
                players: remote.player_count(),
            }
        } else {
            Play::Singleplayer
        };
        let world_name = (!matches!(play, Play::Multiplayer { .. }) && !self.level.name.is_empty())
            .then(|| self.level.name.clone());
        let day = Some((self.world.day_time.max(0) / 24_000) as u64 + 1);
        let hearts = self
            .player
            .as_ref()
            .filter(|p| !p.health.dead_or_dying())
            .map(|p| ((p.health.health() / 2.0).ceil().clamp(0.0, 20.0)) as u8);
        let recording_replay = horizon_client::replay::global_recorder().status().active;
        (
            Status::World {
                play,
                dimension: match self.dim {
                    Dim::Overworld => Dimension::Overworld,
                    Dim::Nether => Dimension::Nether,
                    Dim::End => Dimension::End,
                },
                mode: self.mode,
            },
            PresenceDetails {
                world_name,
                day,
                hearts,
                shaders_active,
                recording_replay,
                screen_recording,
                music_now_playing,
                ..Default::default()
            },
        )
    }
}
