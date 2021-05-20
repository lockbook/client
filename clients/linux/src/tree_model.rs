use glib::translate::*;
use glib::{subclass, Type};
use gtk::prelude::*;
use gtk::subclass::prelude::*;

pub struct LBTreeModelPriv {}

glib::glib_wrapper! {
    pub struct LBTreeModel(
        Object<subclass::simple::InstanceStruct<LBTreeModelPriv>,
        subclass::simple::ClassStruct<LBTreeModelPriv>,
        LBTreeModelClass>)
        @extends gtk::Box, gtk::Container, gtk::Widget;

    match fn {
        get_type => || LBTreeModelPriv::get_type().to_glib(),
    }
}

impl ObjectImpl for LBTreeModelPriv {
    glib::glib_object_impl!();
    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);
        /* ... */
    }

    fn set_property(&self, _obj: &glib::Object, id: usize, value: &glib::Value) {
        let prop = &PROPERTIES[id];
        /* ... */
    }

    fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
        let prop = &PROPERTIES[id];
        Err(())
        /* ... */
    }
}

static PROPERTIES: [subclass::Property; 1] = [subclass::Property("auto-update", |auto_update| {
    glib::ParamSpec::boolean(
        auto_update,
        "Auto-update",
        "Whether to auto-update or not",
        true, // Default value
        glib::ParamFlags::READWRITE,
    )
})];

impl ObjectSubclass for LBTreeModelPriv {
    const NAME: &'static str = "LBTreeModel";
    type ParentType = gtk::Box;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
        klass.add_signal(
            "added",
            glib::SignalFlags::RUN_LAST,
            &[Type::U32],
            Type::Unit,
        );
    }

    fn new() -> Self {
        Self {}
    }
}

impl LBTreeModel {
    pub fn new() -> LBTreeModel {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create MyAwesome Widget")
            .downcast()
            .expect("Created MyAwesome Widget is of wrong type")
    }
}

impl BoxImpl for LBTreeModelPriv {}
impl ContainerImpl for LBTreeModelPriv {}
impl WidgetImpl for LBTreeModelPriv {}
