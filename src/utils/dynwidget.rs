use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;
use rune::{runtime::SyncFunction, Value};
use std::{rc::Rc, sync::Arc};
use zstring::ZString;

/// ImGui widget type.
#[derive(Clone)]
pub enum WidgetType {
    Label(ZString, usize),
    LabelCustomFont(String, Arc<String>),
    Button(ZString, Rc<SyncFunction>, Rc<Option<Value>>),
    Spacing(f32, f32),
    Separator,
    F32Slider(ZString, f32, f32, f32, Rc<SyncFunction>, Rc<Option<Value>>),
    I32Slider(ZString, i32, i32, i32, Rc<SyncFunction>, Rc<Option<Value>>),
    NextWidgetWidth(f32),
    SameLine,
    Image(
        String,
        f32,
        f32,
        bool,
        bool,
        Rc<SyncFunction>,
        Rc<Option<Value>>,
        bool,
    ),
    InputTextMultiLine(
        ZString,
        String,
        f32,
        f32,
        Rc<SyncFunction>,
        Rc<Option<Value>>,
    ),

    /// Advanced widget which hosts more complex widgets, like collapsing headers.
    SubWidget(
        SubWidgetType,
        IndexMap<String, Arc<AtomicRefCell<WidgetType>>>,
        Rc<SyncFunction>,
        Rc<Option<Value>>,
    ),

    Checkbox(ZString, bool, Rc<SyncFunction>, Rc<Option<Value>>),
    ComboBox(
        ZString,
        usize,
        Vec<String>,
        Rc<SyncFunction>,
        Rc<Option<Value>>,
    ),
}

/// Sub-widget type, aka types like collapsing headers and alike.
#[derive(Clone)]
pub enum SubWidgetType {
    CollapsingHeader(ZString),
}
