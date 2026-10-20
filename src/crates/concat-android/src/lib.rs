// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! The activity's entry point. android-activity calls `android_main` on
//! its own thread once the activity is up; Slint's backend takes the
//! activity from there, and the window runs exactly as it does anywhere
//! else.
//!
//! Two things are settled here before the window starts, because only the
//! activity knows them. The app's directories: an Android process has no
//! home directory, so the host's XDG bases are pointed at the app's own
//! files folder, and its external files folder stands in for the home the
//! project and export defaults hang off. And where words go: a process on
//! a phone has no terminal, so the log facade and everything the window
//! prints to stderr are forwarded to logcat under the `concat` tag, which
//! is where `adb logcat -s concat` reads a report from.

/// The activity the app is on now.
///
/// `android_main` can run more than once in a process: the person leaves
/// the app, the system keeps the process, and the launcher brings the app
/// back on it with a new activity. Each call is given its own
/// `AndroidApp`, and the one the first call was given is dead by the
/// second. A hook that keeps the activity it was given talks to a dead
/// one: the picker opens and the answer is delivered to an activity that
/// is gone, so the import does nothing, and keeps doing nothing; the
/// microphone's question is put to nobody. So the current activity is
/// held here, and every hook reads it when it is called rather than
/// keeping the one it was installed with.
#[cfg(target_os = "android")]
static CURRENT: std::sync::Mutex<Option<slint::android::AndroidApp>> = std::sync::Mutex::new(None);

/// Remembers the activity the app is on now.
#[cfg(target_os = "android")]
fn set_current_activity(app: &slint::android::AndroidApp) {
    if let Ok(mut slot) = CURRENT.lock() {
        *slot = Some(app.clone());
    }
}

/// The activity as it is now, for a hook that needs one.
#[cfg(target_os = "android")]
fn current_activity() -> Option<slint::android::AndroidApp> {
    CURRENT.lock().ok().and_then(|slot| slot.clone())
}

#[cfg(target_os = "android")]
mod activity {
    use std::io::BufRead;

    /// Names the app's directories for the host. Set once, before any other
    /// thread exists, which is what makes writing the environment sound.
    pub fn name_directories(app: &slint::android::AndroidApp) {
        let Some(internal) = app.internal_data_path() else {
            return;
        };
        // What the phone keeps for the app: settings, recents, models.
        // SAFETY: called from android_main before the window, the host or
        // any worker thread of ours starts, so no thread of ours reads
        // the environment. The phone's own runtime is already running -
        // it is what called us - and it read the environment it was
        // given when it started, before this call.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &internal);
            std::env::set_var("XDG_DATA_HOME", &internal);
            // Projects and exports land under the folder the user can reach
            // through the phone's file manager, falling back to the private
            // one when there is no external storage.
            let home = app.external_data_path().unwrap_or(internal);
            std::env::set_var("HOME", home);
        }
    }

    /// Routes the log facade, panics and stderr to logcat, and to a file.
    ///
    /// logcat is what a phone on a cable gives you, and nothing else is as
    /// good while there is a cable. A phone in somebody's hand has none, so
    /// the same lines are written into the app's own storage as well; the
    /// window's Settings is where they are found. Called after
    /// [`name_directories`], because that is what says where storage is.
    pub fn open_log() {
        // Info is logcat's floor, as it always was; CONCAT_LOG is the
        // ceiling over both sinks, so turning the file up to debug does not
        // also flood logcat, and turning everything down still quiets it.
        let logcat = android_logger::AndroidLogger::new(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("concat"),
        );
        concat::open_logging(Some(Box::new(logcat)));
        forward_stderr();
    }

    /// Everything written to stderr is read back off a pipe and logged a
    /// line at a time, so a stray `eprintln!` in a dependency reaches logcat
    /// without that dependency knowing about phones. The app's own lines do
    /// not come this way - they go through the facade, which on a phone
    /// deliberately leaves stderr alone so a line cannot come back round
    /// this pipe and log itself forever.
    fn forward_stderr() {
        use std::os::fd::FromRawFd;
        let mut ends = [0i32; 2];
        // SAFETY: plain libc calls on descriptors this function owns; the
        // read end is handed to exactly one File.
        let reader = unsafe {
            if libc::pipe(ends.as_mut_ptr()) != 0 {
                return;
            }
            if libc::dup2(ends[1], libc::STDERR_FILENO) < 0 {
                libc::close(ends[0]);
                libc::close(ends[1]);
                return;
            }
            libc::close(ends[1]);
            std::fs::File::from_raw_fd(ends[0])
        };
        std::thread::Builder::new()
            .name("stderr-to-logcat".into())
            .spawn(move || {
                // Read the bytes, not the lines: a line that is not
                // valid UTF-8 ended the reading, because the iterator
                // stops at the first error, and everything after it was
                // lost - the panic message, the crash handler's line, and
                // every word a dependency wrote. The Android smoke test
                // looks for the panic in there.
                let mut reader = std::io::BufReader::new(reader);
                let mut line = Vec::new();
                loop {
                    line.clear();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let text = String::from_utf8_lossy(&line);
                            log::warn!("{}", text.trim_end());
                        }
                        Err(error) => {
                            log::warn!("stderr: {error}");
                            break;
                        }
                    }
                }
            })
            .ok();
    }
}

/// The Java in java/, and the way to it.
///
/// The window is a NativeActivity with no Java of its own, and some of
/// what a phone does - a picker's answer, the media store - comes only
/// through Java; so the Java is a fragment compiled by build.rs into a dex
/// the binary carries, loaded here through an in-memory class loader
/// once, and reached through [`java::with_class`].
#[cfg(target_os = "android")]
mod java {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    use jni::objects::{Global, JClass, JObject};
    use jni::{Env, JavaVM, jni_sig, jni_str};
    use slint::android::AndroidApp;

    /// The classes build.rs compiled: app.concat.editor.ConcatFiles.
    const DEX: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/classes.dex"));
    const CLASS_NAME: &str = "app.concat.editor.ConcatFiles";

    /// The fragment class, loaded once and kept.
    static CLASS: OnceLock<Global<JClass<'static>>> = OnceLock::new();

    /// Runs `with` on this thread, attached to the activity's JavaVM, with
    /// the activity and the fragment class - loaded the first time.
    pub fn with_class<T>(
        app: &AndroidApp,
        with: impl FnOnce(
            &mut Env<'_>,
            &JObject<'_>,
            &Global<JClass<'static>>,
        ) -> jni::errors::Result<T>,
    ) -> jni::errors::Result<T> {
        // SAFETY: the pointer is the activity's JavaVM, live for the
        // process; `from_raw` also seeds `JavaVM::singleton`, which the
        // native callback reaches for.
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
        vm.attach_current_thread(|env| {
            // SAFETY: the activity pointer is a live reference the app
            // holds for as long as it runs; it is only read here.
            let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
            let class = match CLASS.get() {
                Some(class) => class,
                None => {
                    let loaded = load_class(env, &activity)?;
                    let _ = CLASS.set(loaded);
                    CLASS.get().expect("set just above")
                }
            };
            with(env, &activity, class)
        })
    }

    /// Loads the fragment class from the dex through the activity's own
    /// class loader, and registers `filesPicked` on it.
    fn load_class(
        env: &mut Env,
        activity: &JObject,
    ) -> jni::errors::Result<Global<JClass<'static>>> {
        let parent = env
            .call_method(
                activity,
                jni_str!("getClassLoader"),
                jni_sig!("()Ljava/lang/ClassLoader;"),
                &[],
            )?
            .l()?;
        // SAFETY: DEX is 'static and the loader never writes to it.
        let buffer = unsafe { env.new_direct_byte_buffer(DEX.as_ptr().cast_mut(), DEX.len()) }?;
        let loader = env.new_object(
            jni_str!("dalvik/system/InMemoryDexClassLoader"),
            jni_sig!("(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V"),
            &[(&buffer).into(), (&parent).into()],
        )?;
        let name = env.new_string(CLASS_NAME)?;
        let class = env
            .call_method(
                &loader,
                jni_str!("loadClass"),
                jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"),
                &[(&name).into()],
            )?
            .l()?;
        let class = JClass::cast_local(env, class)?;
        // SAFETY: the signature is the Java declaration's, and the
        // function below takes exactly what it names.
        let method = unsafe {
            jni::NativeMethod::from_raw_parts(
                jni_str!("filesPicked"),
                jni_str!("([Ljava/lang/String;)V"),
                super::picker::files_picked as *mut c_void,
            )
        };
        // SAFETY: as above.
        unsafe { env.register_native_methods(&class, &[method]) }?;
        env.new_global_ref(class)
    }
}

/// The system's document picker.
///
/// A picker's result comes back only through an activity's or a
/// fragment's onActivityResult, so the Java fragment is added to the
/// activity for the length of one pick and told - by the native method
/// registered on it - where to bring the answer. The picked files are
/// copied into the app's own storage by the Java, and their paths are
/// what comes back.
#[cfg(target_os = "android")]
mod picker {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use jni::objects::{JObjectArray, JString};
    use jni::{JavaVM, jni_sig, jni_str, sys};
    use slint::android::AndroidApp;

    type Picked = Box<dyn FnOnce(Vec<PathBuf>) + Send>;

    /// The pick in flight, waiting for Java to answer.
    static PENDING: Mutex<Option<Picked>> = Mutex::new(None);

    /// Hands the window's crate a picker that runs on this activity.
    pub fn install(app: &AndroidApp) {
        super::set_current_activity(app);
        concat::install_file_picker(Box::new(move |kinds, on_picked| {
            // A lock poisoned by an earlier panic still holds the pick:
            // refusing to register this one would leave the window
            // waiting for an answer that goes nowhere.
            let previous = PENDING
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .replace(on_picked);
            if let Some(previous) = previous {
                // A pick was already up; the one that asked first is told
                // it got nothing rather than left waiting for ever.
                previous(Vec::new());
            }
            // The activity as it is now, not the one this closure was
            // given: the app may have been brought back since.
            let app = super::current_activity();
            let Some(app) = app else {
                log::error!("the app has no activity to pick a file with");
                if let Some(pending) = take_pending() {
                    pending(Vec::new());
                }
                return;
            };
            if let Err(error) = pick(&app, &kinds) {
                log::error!("could not show the document picker: {error}");
                if let Some(pending) = take_pending() {
                    pending(Vec::new());
                }
            }
        }));
    }

    /// The pick waiting for an answer, if there is one. A poisoned lock
    /// is not a reason to leave the window waiting for ever.
    fn take_pending() -> Option<Picked> {
        PENDING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    fn pick(app: &AndroidApp, kinds: &concat::PickKinds) -> jni::errors::Result<()> {
        let types = mime_types(kinds).join(",");
        super::java::with_class(app, |env, activity, class| {
            let types = env.new_string(&types)?;
            env.call_static_method(
                class,
                jni_str!("pick"),
                jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)V"),
                &[activity.into(), (&types).into()],
            )?;
            Ok(())
        })
    }

    /// The picker's MIME types for what is asked: media by its families,
    /// and none at all - every file - for anything else. A subtitle or a
    /// font is typed so variously by the phone's providers
    /// (application/x-subrip, text/plain, application/octet-stream...)
    /// that a list would hide the very file asked for; what is picked is
    /// read for what it is, and a wrong file says so.
    pub(super) fn mime_types(kinds: &concat::PickKinds) -> Vec<&'static str> {
        match kinds {
            concat::PickKinds::Media {
                video,
                audio,
                image,
            } => [
                (*video, "video/*"),
                (*audio, "audio/*"),
                (*image, "image/*"),
            ]
            .into_iter()
            .filter_map(|(on, kind)| on.then_some(kind))
            .collect(),
            concat::PickKinds::Files(_) => Vec::new(),
        }
    }

    /// `ConcatFiles.filesPicked`, on whichever thread the Java copied on.
    pub(super) unsafe extern "system" fn files_picked(
        _env: *mut sys::JNIEnv,
        _class: sys::jclass,
        paths: sys::jobjectArray,
    ) {
        // A panic must not leave this frame. It is called from Java, and
        // a Rust panic unwinding into the JVM is an abort: the phone
        // loses the app, and the pick that was waiting for an answer
        // waits for ever. So the reading is caught, the answer is still
        // given - empty, if it could not be read - and the window's own
        // callback is caught too.
        let read = JavaVM::singleton().and_then(|vm| {
            vm.attach_current_thread(|env| {
                // SAFETY: `paths` is the argument Java handed this frame.
                let array = unsafe { JObjectArray::<JString>::from_raw(env, paths) };
                let mut out = Vec::new();
                for index in 0..array.len(env)? {
                    let item = array.get_element(env, index)?;
                    out.push(PathBuf::from(item.mutf8_chars(env)?.to_string()));
                }
                Ok::<_, jni::errors::Error>(out)
            })
        });
        let paths = match read {
            Ok(paths) => paths,
            Err(error) => {
                log::error!("could not read the picked files: {error}");
                Vec::new()
            }
        };
        // The lock may be poisoned by a panic in an earlier pick, and a
        // poisoned lock is not a reason to leave the window waiting.
        if let Some(pending) = take_pending() {
            let given = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pending(paths)));
            if given.is_err() {
                log::error!("the pick callback panicked");
            }
        }
    }
}

/// Where a finished export goes.
///
/// The export writes into the app's own folder, which on a phone nothing
/// but the app can open: not the gallery, not a file manager (#280,
/// #147). So once a file is written the Java moves it into the phone's
/// Movies, under Concat, through the media store, and the sheet names
/// that folder from the start. A phone older than Android 10 has no
/// media store an app can write through without a permission; there the
/// file stays where it was written, and the sheet says so.
#[cfg(target_os = "android")]
mod publisher {
    use std::path::Path;

    use jni::objects::{JObjectArray, JString};
    use jni::refs::Reference;
    use jni::{jni_sig, jni_str};
    use slint::android::AndroidApp;

    /// Hands the window's crate the way to the media store, where this
    /// phone has one to write through.
    pub fn install(app: &AndroidApp) {
        let folder = match published_folder(app) {
            Ok(Some(folder)) => folder,
            Ok(None) => {
                log::info!(
                    "exports stay in the app's folder: this phone's media store takes no files"
                );
                return;
            }
            Err(error) => {
                log::warn!("could not ask where exports go: {error}");
                return;
            }
        };
        let app = app.clone();
        let documents = app.clone();
        concat::install_export_publisher(concat::ExportPublisher {
            folder,
            publish: Box::new(move |path| publish(&app, path)),
            publish_document: Box::new(move |path, mime| publish_document(&documents, path, mime)),
        });
    }

    /// `ConcatFiles.publishedFolder`: "Movies/Concat", or nothing.
    fn published_folder(app: &AndroidApp) -> jni::errors::Result<Option<String>> {
        super::java::with_class(app, |env, _activity, class| {
            let answer = env
                .call_static_method(
                    class,
                    jni_str!("publishedFolder"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            if answer.is_null() {
                return Ok(None);
            }
            let answer = env.cast_local::<JString>(answer)?;
            Ok(Some(answer.mutf8_chars(env)?.to_string()))
        })
    }

    /// `ConcatFiles.publishVideo`: the file's path under the phone's
    /// storage once it is in Movies, or why it could not be moved.
    fn publish(app: &AndroidApp, path: &Path) -> Result<String, String> {
        answer(super::java::with_class(app, |env, activity, class| {
            let path = env.new_string(path.to_string_lossy())?;
            let answer = env
                .call_static_method(
                    class,
                    jni_str!("publishVideo"),
                    jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)[Ljava/lang/String;"),
                    &[activity.into(), (&path).into()],
                )?
                .l()?;
            strings(env, answer)
        }))
    }

    /// `ConcatFiles.publishDocument`: the file's path under the phone's
    /// storage once it is in Download/Concat, or why it could not be moved.
    fn publish_document(app: &AndroidApp, path: &Path, mime: &str) -> Result<String, String> {
        answer(super::java::with_class(app, |env, activity, class| {
            let path = env.new_string(path.to_string_lossy())?;
            let mime = env.new_string(mime)?;
            let answer = env
                .call_static_method(
                    class,
                    jni_str!("publishDocument"),
                    jni_sig!("(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)[Ljava/lang/String;"),
                    &[activity.into(), (&path).into(), (&mime).into()],
                )?
                .l()?;
            strings(env, answer)
        }))
    }

    /// A Java `String[]` as Rust's, a null element as None.
    fn strings<'local>(
        env: &mut jni::Env<'local>,
        answer: jni::objects::JObject<'local>,
    ) -> jni::errors::Result<Vec<Option<String>>> {
        let answer = env.cast_local::<JObjectArray<JString>>(answer)?;
        let mut out = Vec::new();
        for index in 0..answer.len(env)? {
            let item = answer.get_element(env, index)?;
            out.push(if item.is_null() {
                None
            } else {
                Some(item.mutf8_chars(env)?.to_string())
            });
        }
        Ok(out)
    }

    /// What `publishVideo` and `publishDocument` answer: where the file is
    /// now, or why it is still where it was.
    fn answer(answer: jni::errors::Result<Vec<Option<String>>>) -> Result<String, String> {
        let answer =
            answer.map_err(|error| format!("could not reach the phone's media store: {error}"))?;
        match answer.as_slice() {
            [Some(path), _] => Ok(path.clone()),
            [None, Some(why)] => Err(why.clone()),
            _ => Err("the phone said nothing".to_owned()),
        }
    }
}

/// The microphone, for a voiceover.
///
/// Android grants it while the app runs, not at install, and an input
/// stream opened without it hears only silence; so the voiceover panel
/// asks here before it opens one, and the Java puts the system's question
/// up when the answer is not yet yes.
#[cfg(target_os = "android")]
mod microphone {
    use jni::{jni_sig, jni_str};
    use slint::android::AndroidApp;

    /// Hands the window's crate the way to ask.
    pub fn install(app: &AndroidApp) {
        super::set_current_activity(app);
        concat::install_microphone_access(Box::new(move || {
            // The activity as it is now: the question is put to the
            // activity the app is on, and a dead one never asks.
            let Some(app) = super::current_activity() else {
                log::warn!("the app has no activity to ask for the microphone with");
                return true;
            };
            super::java::with_class(&app, |env, activity, class| {
                env.call_static_method(
                    class,
                    jni_str!("askMicrophone"),
                    jni_sig!("(Landroid/app/Activity;)Z"),
                    &[activity.into()],
                )?
                .z()
            })
            .unwrap_or_else(|error| {
                // Not knowing is not a no: the device is opened, and its
                // own error, or its silence, says the rest.
                log::warn!("could not ask for the microphone: {error}");
                true
            })
        }));
    }
}

/// The browser, for the launch screen's guides and links: an ACTION_VIEW
/// intent through the activity, which the Java sends.
#[cfg(target_os = "android")]
mod links {
    use jni::objects::JString;
    use jni::{jni_sig, jni_str};
    use slint::android::AndroidApp;

    /// Hands the window's crate the way to open a web address.
    pub fn install(app: &AndroidApp) {
        super::set_current_activity(app);
        concat::install_link_opener(Box::new(move |url| {
            let Some(app) = super::current_activity() else {
                return Err("the app has no activity to open a link with".to_owned());
            };
            let answer = super::java::with_class(&app, |env, activity, class| {
                let url = env.new_string(url)?;
                let answer = env
                    .call_static_method(
                        class,
                        jni_str!("openLink"),
                        jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;"),
                        &[activity.into(), (&url).into()],
                    )?
                    .l()?;
                if answer.is_null() {
                    return Ok(None);
                }
                let answer = env.cast_local::<JString>(answer)?;
                Ok(Some(answer.mutf8_chars(env)?.to_string()))
            });
            match answer {
                Ok(None) => Ok(()),
                Ok(Some(why)) => Err(why),
                Err(error) => Err(format!("could not reach the browser: {error}")),
            }
        }));
    }
}

/// Called by the activity's native glue; the name is the contract.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    // Directories first: the log file is written into one of them.
    activity::name_directories(&app);
    activity::open_log();
    log::info!("Concat {} starting", env!("CARGO_PKG_VERSION"));
    if let Err(error) = slint::android::init(app.clone()) {
        log::error!("could not start the Android backend: {error}");
        return;
    }
    // After the backend, which seeds the JavaVM the Java is reached by.
    picker::install(&app);
    publisher::install(&app);
    microphone::install(&app);
    links::install(&app);
    if let Err(error) = concat::run() {
        log::error!("{error}");
    }
}
