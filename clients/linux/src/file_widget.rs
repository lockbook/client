use glib::{subclass, Type, Value, GString, SignalHandlerId};
use glib::translate::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{TreeModel as GtkTreeModel, Widget, TreeIter, TreeStore, TreeModel, TreePath, TreeModelFlags};
use gtk::TreeStore as GtkTreeStore;
use gtk::TreeView as GtkTreeView;
use std::os::raw::c_int;

static PROPERTIES: [subclass::Property; 0] = [];

pub struct FileTreeModelPriv {}

glib::glib_wrapper! {
    pub struct FileTreeModel(
        Object<subclass::simple::InstanceStruct<FileTreeModelPriv>,
        subclass::simple::ClassStruct<FileTreeModelPriv>,
        FileTreeModelClass>)
        @implements gtk::Buildable, gtk::TreeDragDest, gtk::TreeDragSource, gtk::TreeModel, gtk::TreeSortable;

    match fn {
        get_type => || FileTreeModelPriv::get_type().to_glib(),
    }
}

impl ObjectImpl for FileTreeModelPriv {
    glib::glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj)
    }

    fn set_property(&self, _obj: &glib::Object, id: usize, value: &glib::Value) {
    }

    fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
        Err(())
    }
}

impl ObjectSubclass for FileTreeModelPriv {
    const NAME: &'static str = "FileTreeModel";
    type ParentType = glib::Object;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        Self {}
    }
}

impl FileTreeModel {
    pub fn new(column_types: &[Type]) -> Self {
        unsafe {
            let mut column_types = column_types.iter().map(|t| t.to_glib()).collect::<Vec<_>>();
            from_glib_full(gtk_sys::gtk_tree_store_newv(
                column_types.len() as c_int,
                column_types.as_mut_ptr(),
            ))
        }
    }

    pub fn clear(&self) {
        unsafe {
            gtk_sys::gtk_tree_store_clear(self.as_ref().to_glib_none().0);
        }
    }

    pub fn insert_with_values(
        &self,
        parent: Option<&TreeIter>,
        position: Option<u32>,
        columns: &[u32],
        values: &[&dyn ToValue],
    ) -> TreeIter {
        unsafe {
            assert!(position.unwrap_or(0) <= i32::max_value() as u32);
            assert_eq!(columns.len(), values.len());
            let n_columns = gtk_sys::gtk_tree_model_get_n_columns(
                self.as_ref().upcast_ref::<TreeModel>().to_glib_none().0,
            ) as u32;
            assert!(columns.len() <= n_columns as usize);
            for (&column, value) in columns.iter().zip(values.iter()) {
                let type_ = from_glib(gtk_sys::gtk_tree_model_get_column_type(
                    self.as_ref().upcast_ref::<TreeModel>().to_glib_none().0,
                    column as c_int,
                ));
                assert!(Value::type_transformable(value.to_value_type(), type_));
            }
            let mut iter = TreeIter::uninitialized();
            gtk_sys::gtk_tree_store_insert_with_valuesv(
                self.as_ref().to_glib_none().0,
                iter.to_glib_none_mut().0,
                mut_override(parent.to_glib_none().0),
                position.map_or(-1, |n| n as c_int),
                mut_override(columns.as_ptr() as *const c_int),
                values.to_glib_none().0,
                columns.len() as c_int,
            );
            iter
        }
    }

    pub fn set(&self, iter: &TreeIter, columns: &[u32], values: &[&dyn ToValue]) {
        unsafe {
            assert_eq!(columns.len(), values.len());
            let n_columns = gtk_sys::gtk_tree_model_get_n_columns(
                self.as_ref().upcast_ref::<TreeModel>().to_glib_none().0,
            ) as u32;
            assert!(columns.len() <= n_columns as usize);
            for (&column, value) in columns.iter().zip(values.iter()) {
                assert!(column < n_columns);
                let type_ = from_glib(gtk_sys::gtk_tree_model_get_column_type(
                    self.as_ref().upcast_ref::<TreeModel>().to_glib_none().0,
                    column as c_int,
                ));
                assert!(Value::type_transformable(value.to_value_type(), type_));
            }
            gtk_sys::gtk_tree_store_set_valuesv(
                self.as_ref().to_glib_none().0,
                mut_override(iter.to_glib_none().0),
                mut_override(columns.as_ptr() as *const c_int),
                values.to_glib_none().0,
                columns.len() as c_int,
            );
        }
    }

    pub fn remove(&self, iter: &TreeIter) -> bool {
        unsafe {
            from_glib(gtk_sys::gtk_tree_store_remove(
                self.as_ref().to_glib_none().0,
                mut_override(iter.to_glib_none().0),
            ))
        }
    }
}


