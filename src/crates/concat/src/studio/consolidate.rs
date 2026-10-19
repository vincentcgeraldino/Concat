// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! Selection consolidation: render first, then replace in one undo step.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use concat_host::consolidate::Consolidation;
use concat_project::model::{Clip, Timeline};
use concat_project::{consolidation_clips, why_not_merge};

use super::*;

pub(super) struct MergeJob {
    cancel: Arc<AtomicBool>,
    progress: f32,
}

/// Removes an unadopted render even when its project was closed before
/// the worker's callback arrived. Successful edits keep the generated media
/// through undo/redo and across cache clearing.
struct RenderedFile {
    folder: PathBuf,
    keep: bool,
}

impl Drop for RenderedFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(self.folder.join("merged.mp4"));
            let _ = std::fs::remove_dir(&self.folder);
        }
    }
}

impl Studio {
    pub fn merge_blocked(&self) -> Option<String> {
        if self.merge_job.is_some() {
            return None; // The same action cancels a running merge.
        }
        if self.host.exporter.is_busy() {
            return Some(t("studio.mergeExportBusy"));
        }
        if self.selection.iter().any(|id| {
            self.clip(id)
                .is_some_and(|clip| self.locked(&clip.track_id))
        }) {
            return Some(t("studio.mergeTrackLocked"));
        }
        consolidation_clips(self.timeline(), &self.selection)
            .err()
            .map(|error| error.to_string())
    }

    pub(super) fn merge_label(&self) -> String {
        match &self.merge_job {
            Some(job) => tf(
                "studio.mergeCancelProgress",
                &[&format!("{:.0}", job.progress * 100.0)],
            ),
            None => t("studio.mergeSelected"),
        }
    }

    pub(super) fn cancel_merge(&mut self) {
        if let Some(job) = self.merge_job.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn merge(&mut self) {
        if let Some(job) = &self.merge_job {
            job.cancel.store(true, Ordering::Relaxed);
            return;
        }
        self.flush_commit();
        if let Some(reason) = self.merge_blocked() {
            self.notify(&reason, true);
            return;
        }
        // Pieces of one file that still touch and carry nothing a rejoin
        // would bend go back together without a render: nothing to encode,
        // nothing lost (audit M5 - a fresh split's halves are the everyday
        // case). Everything else consolidates through a rendered file below.
        if why_not_merge(self.timeline(), &self.selection).is_none()
            && rejoin_is_faithful(self.timeline(), &self.selection)
        {
            let ids = self.selection.clone();
            if let Some(id) = self.apply(Command::MergeClips { clip_ids: ids }) {
                self.selection = vec![id];
                self.transition_selected = None;
                self.request_preview();
                self.notify(&t("studio.mergeFinished"), false);
                return;
            }
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let plan = match Consolidation::new(session.project(), &self.selection) {
            Ok(plan) => plan,
            Err(error) => {
                self.notify(&error, true);
                return;
            }
        };
        let job = match self.host.exporter.begin() {
            Ok(job) => job,
            Err(error) => {
                self.notify(&error, true);
                return;
            }
        };
        let root = PathBuf::from(session.path());
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let folder = root.join("media").join(format!("merged-{stamp}"));
        if let Err(error) =
            std::fs::create_dir_all(root.join("media")).and_then(|()| std::fs::create_dir(&folder))
        {
            self.notify(&tf("studio.mergeFailed", &[&error.to_string()]), true);
            return;
        }
        let rendered = RenderedFile {
            folder: folder.clone(),
            keep: false,
        };
        let output = folder.join("merged.mp4");
        let request = plan.request(session.project(), &root, &output);
        let compositor = self.host.monitor.off_thread();
        self.merge_job = Some(MergeJob {
            cancel: job.cancel_handle(),
            progress: 0.0,
        });
        self.pause();
        self.notify(&t("studio.mergeStarted"), false);
        let epoch = crate::host::project_epoch();
        spawn_in_project(
            move || {
                let mut last = -1.0;
                let result = concat_host::export::run_on(
                    &request,
                    compositor,
                    job.cancel_flag(),
                    |progress| {
                        let fraction =
                            (progress.frame as f32 / progress.total.max(1) as f32).clamp(0.0, 1.0);
                        if fraction - last >= 0.02 {
                            last = fraction;
                            on_ui_in_project(epoch, move |studio, _, _| {
                                if let Some(job) = studio.merge_job.as_mut() {
                                    job.progress = fraction;
                                }
                            });
                        }
                    },
                );
                let cancel = job.cancel_handle();
                (plan, rendered, output, cancel, result)
            },
            |studio, _, _, (plan, mut rendered, output, cancel, result)| {
                studio.merge_job = None;
                if cancel.load(Ordering::Relaxed) {
                    studio.notify(&t("studio.mergeCancelled"), false);
                    return;
                }
                let outcome = result.and_then(|_| {
                    let summary = media::probe(&output.to_string_lossy())?;
                    let mut item = summary.to_new_media();
                    item.name = t("studio.mergedClips");
                    item.origin = Some(model::MediaOrigin::Processed);
                    studio.flush_commit();
                    let command = plan.replacement(studio.project(), item)?;
                    // Locking a lane while rendering must also prevent adoption.
                    if studio.locked(plan.track_id()) {
                        return Err(t("studio.mergeTrackLocked"));
                    }
                    studio
                        .apply(command)
                        .ok_or_else(|| t("studio.mergeCouldNotReplace"))
                });
                match outcome {
                    Ok(id) => {
                        rendered.keep = true;
                        studio.selection = vec![id];
                        studio.transition_selected = None;
                        studio.request_media_art();
                        studio.request_preview();
                        studio.notify(&t("studio.mergeFinished"), false);
                    }
                    Err(error) => studio.notify(&tf("studio.mergeFailed", &[&error]), true),
                }
            },
        );
    }
}

/// Whether re-joining these clips is faithful as well as lossless: they
/// must be pieces of one clip cut into windows - every setting alike on
/// every piece, with only what a split leaves different and a merge puts
/// back: the spans, the keyframe tracks (which absorb), the first piece's
/// fade-in and incoming transition, and the last piece's fade-out. A piece
/// carrying its own gain, effect, mid-way fade or transition would be bent
/// by the rejoin, so those selections render instead (audit M5).
fn rejoin_is_faithful(timeline: &Timeline, clip_ids: &[String]) -> bool {
    let mut pieces: Vec<&Clip> = clip_ids.iter().filter_map(|id| timeline.clip(id)).collect();
    pieces.sort_by(|left, right| left.start.total_cmp(&right.start));
    let Some((first, rest)) = pieces.split_first() else {
        return false;
    };
    // What the merge itself rewrites is blanked out of both sides of the
    // comparison; everything else must match the first piece.
    let unmoved = |clip: &Clip| {
        let mut clip = clip.clone();
        clip.id = String::new();
        clip.name = String::new();
        clip.start = 0.0;
        clip.duration = 0.0;
        clip.source_start = 0.0;
        clip.keys = Vec::new();
        clip.fade_in = 0.0;
        clip.fade_out = 0.0;
        clip.transition_in = None;
        clip
    };
    // The head's own fade-out and the last piece's fade-in are replaced by
    // the other end's, so they must be nothing to begin with.
    if first.fade_out != 0.0 {
        return false;
    }
    let head = unmoved(first);
    for (index, piece) in rest.iter().enumerate() {
        let last = index + 1 == rest.len();
        if piece.fade_in != 0.0 || (!last && piece.fade_out != 0.0) {
            return false;
        }
        if piece.transition_in.is_some() {
            return false;
        }
        if unmoved(piece) != head {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use concat_project::commands::{ClipPatch, NewMedia};
    use concat_project::model::MediaKind;
    use concat_project::{Command, Editor};

    #[test]
    fn unadopted_render_is_removed_but_adopted_media_survives() {
        let folder =
            std::env::temp_dir().join(format!("concat-merge-cleanup-{}", std::process::id()));
        std::fs::create_dir(&folder).unwrap();
        let output = folder.join("merged.mp4");
        std::fs::write(&output, b"rendered source").unwrap();
        drop(RenderedFile {
            folder: folder.clone(),
            keep: false,
        });
        assert!(
            !folder.exists(),
            "cancel, failure and stale delivery clean up the source"
        );
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(&output, b"adopted source").unwrap();
        drop(RenderedFile {
            folder: folder.clone(),
            keep: true,
        });
        assert!(output.is_file(), "undo/redo still needs adopted media");
        std::fs::remove_file(output).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }

    /// An editor with one ten-second clip split at four seconds; the
    /// pieces' ids come back in timeline order.
    fn split_pair() -> (Editor, Vec<String>) {
        let mut editor = Editor::new();
        let item = NewMedia {
            path: "/a.mp4".to_owned(),
            name: "a.mp4".to_owned(),
            duration: Some(10.0),
            kind: MediaKind::Video,
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(30.0),
            frame_rate_fraction: Some("30/1".to_owned()),
            video_codec: Some("h264".to_owned()),
            audio_codec: None,
            has_audio: false,
            audio_tracks: Vec::new(),
            origin: None,
            color_space: Default::default(),
        };
        let media_id = editor
            .apply(Command::AddMedia { item })
            .expect("adds the media")
            .created_id
            .expect("the media is minted");
        let track_id = editor.project().active().tracks[0].id.clone();
        let whole = editor
            .apply(Command::AddClip {
                media_id,
                track_id,
                start: 0.0,
                ripple: false,
            })
            .expect("adds the clip")
            .created_id
            .expect("the clip is minted");
        editor
            .apply(Command::SplitClips {
                clip_ids: vec![whole],
                time: 4.0,
            })
            .expect("splits");
        let ids = editor
            .project()
            .active()
            .clips
            .iter()
            .map(|clip| clip.id.clone())
            .collect();
        (editor, ids)
    }

    /// A fresh split's halves rejoin losslessly: the pieces differ only in
    /// the windows the split rewound, which the merge puts back.
    #[test]
    fn a_fresh_split_rejoins_losslessly() {
        let (editor, ids) = split_pair();
        let timeline = editor.project().active();
        assert!(
            why_not_merge(timeline, &ids).is_none(),
            "the shape is joinable"
        );
        assert!(
            rejoin_is_faithful(timeline, &ids),
            "and nothing in it would be bent"
        );
    }

    /// A piece with an edit of its own - a fade or a gain the cut did not
    /// leave on both halves - would lose it to a rejoin, so that selection
    /// renders instead.
    #[test]
    fn a_piece_with_its_own_edit_renders_instead() {
        let cases: [(&str, ClipPatch); 3] = [
            (
                "a fade-in in the middle",
                ClipPatch {
                    fade_in: Some(0.5),
                    ..ClipPatch::default()
                },
            ),
            (
                "a gain only the tail carries",
                ClipPatch {
                    volume: Some(0.5),
                    ..ClipPatch::default()
                },
            ),
            (
                "a mute",
                ClipPatch {
                    muted: Some(true),
                    ..ClipPatch::default()
                },
            ),
        ];
        for (what, patch) in cases {
            let (mut editor, ids) = split_pair();
            editor
                .apply(Command::UpdateClip {
                    clip_id: ids[1].clone(),
                    patch,
                })
                .expect("patches");
            assert!(
                !rejoin_is_faithful(editor.project().active(), &ids),
                "{what} would be bent"
            );
        }
    }

    /// The head's own fade-out is replaced by the last piece's in a rejoin,
    /// so an edit there renders too.
    #[test]
    fn the_heads_own_fade_out_renders_instead() {
        let (mut editor, ids) = split_pair();
        editor
            .apply(Command::UpdateClip {
                clip_id: ids[0].clone(),
                patch: ClipPatch {
                    fade_out: Some(0.5),
                    ..ClipPatch::default()
                },
            })
            .expect("patches");
        assert!(!rejoin_is_faithful(editor.project().active(), &ids));
    }
}
