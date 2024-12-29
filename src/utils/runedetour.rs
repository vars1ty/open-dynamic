use super::scripting::script_core::ValueWrapper;
use crate::utils::extensions::{OptionExt, ResultExtensions};
use parking_lot::{Once, RwLock};
use retour::RawDetour;
use rune::{
    runtime::{Function, SyncFunction},
    Value,
};
use std::sync::{atomic::AtomicUsize, Arc};

/// Amount of parameters to collect from hooked functions.
pub static COLLECT_PARAMS_COUNT: AtomicUsize = const { AtomicUsize::new(10) };

/// All Rune detours, acquired and non-acquired.
static RUNE_DETOURS: RwLock<Vec<Arc<RwLock<RDetour>>>> = const { RwLock::new(Vec::new()) };

/// Generates code for the unique ID tied to the calling function, and collects 10 arguments from
/// `args` into `args_out`.
/// The ID is manual and **must** be unique!
/// `call_once` is as the name implies; Only called once, which is when `register_all_detours` is
/// called.
/// Leave as `{}` for no code to be executed.
macro_rules! generate_detour_id {
    ($id:literal, $args:expr) => {{
        static CALL_ONCE: Once = Once::new();
        CALL_ONCE.call_once(|| {
            RDetour::register_new_detour($id);
        });

        let mut collected_args =
            Vec::with_capacity(COLLECT_PARAMS_COUNT.load(std::sync::atomic::Ordering::Relaxed));
        for _ in 0..collected_args.capacity() {
            collected_args.push(unsafe { $args.arg::<*const i64>() } as i64);
        }

        RDetour::call_rune_function_on_id($id, collected_args) as *const i64
    }};
}

/// Generates a detour holder (target) function, then calls the `generate_detour_id!()` macro
/// inside it.
macro_rules! generate_detour_holder {
    ($fn_name:ident, $id:literal) => {
        unsafe extern "C" fn $fn_name(mut args: ...) -> *const i64 {
            generate_detour_id!($id, args)
        }
    };
}

/// Holds the information about a Rune detour.
#[derive(Default)]
pub struct RDetour {
    /// The ID of the detour.
    detour_id: u8,

    /// The pointer of which function will be treated as target, and will be redirected to a
    /// determined detour holder from `determine_detour_holder()`.
    /// If `None`, this detour isn't ready to be used and is free to be acquired.
    from_ptr: Option<i64>,

    /// The `RawDetour` instance.
    /// If `None`, this detour isn't ready to be used and is free to be acquired.
    detour: Option<Box<RawDetour>>,

    /// Rune function to be called as a callback, should return a `i64` of the original functions
    /// return value as a pointer, or a modified value if needed.
    /// If `None`, this detour isn't ready to be used and is free to be acquired.
    rune_function: Option<SyncFunction>,

    /// Optional paramater to be passed into `rune_function` upon callback.
    /// Can be a structure for example, so that variables can be updated.
    opt_param: Option<ValueWrapper>,
}

impl RDetour {
    /// Calls all `detour_holder_xx` functions in order for each one to register itself via `register_new_detour`.
    pub fn register_all_detours() {
        unsafe {
            detour_holder_00();
            detour_holder_01();
            detour_holder_02();
            detour_holder_03();
            detour_holder_04();
            detour_holder_05();
            detour_holder_06();
            detour_holder_07();
            detour_holder_08();
            detour_holder_09();
        }

        #[cfg(target_pointer_width = "32")]
        log!("[WARN] RDetours ready, note that 32-bit is not stable with c_variadic-forced hooks!");
    }

    /// Registers a new detour at ID `detour_id - 1` in `RUNE_DETOURS`.
    /// Note that this only **registers** it, the detour instance is **not** created here.
    fn register_new_detour(detour_id: u8) {
        let Some(mut rune_detours) = RUNE_DETOURS.try_write() else {
            log!("[ERROR] rune_detours is locked, cannot register!");
            return;
        };

        if rune_detours.iter().any(|rdetour| {
            rdetour
                .try_read()
                .unwrap_or_crash(zencstr!(
                    "[ERROR] RDetour is locked, cannot safely add ID ",
                    detour_id,
                    "!"
                ))
                .get_detour_id()
                == detour_id
        }) {
            crash!(
                "[ERROR] RDetour with ID ",
                detour_id,
                " already exists, read how to properly use `generate_detour_id!()`!"
            );
        }

        rune_detours.insert(
            detour_id as usize,
            Arc::new(RwLock::new(Self {
                detour_id,
                ..Default::default()
            })),
        );
    }

    /// Automatically finds the first-available `RDetour` and installs it on `from_ptr` with the
    /// callback function of `rune_function`.
    pub fn install_detour_auto(from_ptr: i64, rune_function: Function, opt_param: Option<Value>) {
        let rune_function = rune_function.into_sync().into_result();
        if let Err(error) = rune_function {
            log!(
                "[ERROR] Failed turning Rune function into SyncFunction, error: ",
                error
            );
            return;
        };

        let rune_function = rune_function.unwrap();
        let Some(available_detour) = Self::find_free_detour() else {
            log!("[ERROR] All RDetours are busy!");
            return;
        };

        available_detour
            .try_write()
            .unwrap_or_crash(zencstr!(
                "[ERROR] The found RDetour is locked and cannot be modified!"
            ))
            .install_detour(from_ptr, rune_function, opt_param);
    }

    /// Finds the first-available `RDetour` and returns it.
    fn find_free_detour() -> Option<Arc<RwLock<Self>>> {
        let Some(rune_detours) = RUNE_DETOURS.try_read() else {
            log!("[ERROR] rune_detours is locked, cannot find free detours!");
            return None;
        };

        Some(Arc::clone(
            rune_detours
                .iter()
                .find(|rdetour| !rdetour.read().is_detour_acquired())?,
        ))
    }

    /// Installs a detour from `from_ptr` into a freely-available detour holder function, which
    /// calls `rune_function`.
    fn install_detour(
        &mut self,
        from_ptr: i64,
        rune_function: SyncFunction,
        opt_param: Option<Value>,
    ) {
        if self.is_detour_acquired() {
            log!(
                "[ERROR] The RDetour of ID ",
                self.detour_id,
                " has already been acquired!"
            );
            return;
        }

        let to_ptr = self.determine_detour_holder();
        self.rune_function = Some(rune_function);
        self.from_ptr = Some(from_ptr);
        self.opt_param = opt_param.map(ValueWrapper);

        unsafe {
            let hook = Self::create_hook(from_ptr as *const (), to_ptr);
            if let Err(error) = hook.enable() {
                log!(
                    "[ERROR] Failed enabling RDetour at ID ",
                    self.get_detour_id(),
                    ", error: ",
                    error
                );
                return;
            }
            self.detour = Some(hook);
        }

        log!(
            "[RDetour] RDetour with ID ",
            self.get_detour_id(),
            " enabled!"
        );
    }

    /// Calls the associated Rune function on `detour_id`, passing in the original function pointer
    /// and `args`.
    /// Returns 0 (null) if no function is associated or if there's an error.
    fn call_rune_function_on_id(detour_id: u8, args: Vec<i64>) -> i64 {
        let Some(rune_detours) = RUNE_DETOURS.try_read() else {
            log!("[ERROR] rune_detours is locked, cannot call Rune function!");
            return 0;
        };

        let Some(rdetour) = rune_detours
            .iter()
            .find(|rdetour| rdetour.read().get_detour_id() == detour_id)
        else {
            return 0;
        };

        let Some(rdetour) = rdetour.try_read() else {
            log!(
                "[ERROR] RDetour at ID ",
                detour_id,
                " is locked, cannot call Rune function!"
            );
            return 0;
        };

        let Some(rune_function) = rdetour.get_rune_function() else {
            return 0;
        };

        let Some(detour) = rdetour.get_raw_detour() else {
            log!("[ERROR] Missing RawDetour for ID ", detour_id, "!");
            return 0;
        };

        let original = detour.trampoline() as *const ();
        let call_res = rune_function
            .call::<(i64, Vec<i64>, Option<&Value>), i64>((
                original as _,
                args,
                rdetour.opt_param.as_ref().map(|value| &value.0),
            ))
            .into_result();
        let Err(error) = call_res else {
            return call_res.dynamic_expect(zencstr!("Safety check for error failed?"));
        };

        log!(
            "[ERROR] Failed calling Rune function on ID ",
            detour_id,
            ", error: ",
            error
        );
        0
    }

    /// Determines the `detour_holder_xx` function based on `self.get_detour_id()` and returns it
    /// as a pointer.
    fn determine_detour_holder(&self) -> *const () {
        match self.get_detour_id() {
            0 => detour_holder_00 as *const (),
            1 => detour_holder_01 as *const (),
            2 => detour_holder_02 as *const (),
            3 => detour_holder_03 as *const (),
            4 => detour_holder_04 as *const (),
            5 => detour_holder_05 as *const (),
            6 => detour_holder_06 as *const (),
            7 => detour_holder_07 as *const (),
            8 => detour_holder_08 as *const (),
            9 => detour_holder_09 as *const (),
            _ => crash!(
                "[ERROR] RDetour at ID ",
                self.get_detour_id(),
                " doesn't have any reserved function for it!"
            ),
        }
    }

    /// Creates a new hook from a pointer, to another.
    /// The inner function is always redirected into a c_variadic function to grab the arguments.
    /// This may cause UB and should be used with **extreme** care!
    fn create_hook(from: *const (), to: *const ()) -> Box<RawDetour> {
        unsafe {
            let hook = RawDetour::new(from, to).dynamic_unwrap();
            hook.enable().dynamic_unwrap();
            Box::new(hook)
        }
    }

    /// Drops a detour from `address` if there's any installed RDetours at that address.
    pub fn drop_rdetour_at(address: i64) -> bool {
        let address = address as *const i64;
        let Some(rune_detours) = RUNE_DETOURS.try_read() else {
            log!("[ERROR] rune_detours is locked, cannot access RDetours!");
            return false;
        };

        let Some(rdetour) = rune_detours.iter().find(|rdetour| {
            rdetour
                .try_read()
                .unwrap_or_crash(zencstr!(
                    "[ERROR] RDetour at address ",
                    format!("{address:?}"),
                    " is locked and cannot be modified!"
                ))
                .get_from_address()
                .unwrap_or_default()
                == address as i64
        }) else {
            return false;
        };

        let Some(mut rdetour) = rdetour.try_write() else {
            log!(
                "[ERROR] RDetour at ",
                format!("{address:?}"),
                " is locked and cannot be modified!"
            );
            return false;
        };

        let Some(detour) = rdetour.detour.take() else {
            log!(
                "[ERROR] Couldn't obtain RawDetour from ID ",
                rdetour.get_detour_id(),
                ", address ",
                format!("{address:?}"),
                "!"
            );
            return false;
        };

        if detour.is_enabled()
            && let Err(error) = unsafe { detour.disable() }
        {
            log!(
                "[ERROR] Failed disabling RDetour at ID ",
                rdetour.get_detour_id(),
                ", address ",
                format!("{address:?}"),
                ", error: ",
                error
            );
            return false;
        }

        rdetour.from_ptr = None;
        drop(rdetour.rune_function.take());
        drop(detour);
        true
    }

    /// Returns `self.from_ptr`.
    const fn get_from_address(&self) -> &Option<i64> {
        &self.from_ptr
    }

    /// Returns `self.retour`.
    const fn get_raw_detour(&self) -> &Option<Box<RawDetour>> {
        &self.detour
    }

    /// Returns `self.rune_function`.
    const fn get_rune_function(&self) -> &Option<SyncFunction> {
        &self.rune_function
    }

    /// Retuns `self.detour_id`.
    const fn get_detour_id(&self) -> u8 {
        self.detour_id
    }

    /// Returns `true` if this detour has been acquired, `false` if it hasn't.
    /// If it has, it is **not** available for `install_detour*`.
    const fn is_detour_acquired(&self) -> bool {
        self.get_from_address().is_some() || self.get_raw_detour().is_some()
    }
}

generate_detour_holder!(detour_holder_00, 0);
generate_detour_holder!(detour_holder_01, 1);
generate_detour_holder!(detour_holder_02, 2);
generate_detour_holder!(detour_holder_03, 3);
generate_detour_holder!(detour_holder_04, 4);
generate_detour_holder!(detour_holder_05, 5);
generate_detour_holder!(detour_holder_06, 6);
generate_detour_holder!(detour_holder_07, 7);
generate_detour_holder!(detour_holder_08, 8);
generate_detour_holder!(detour_holder_09, 9);
