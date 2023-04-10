use crate::helper::{
  create_database_with_default_data, DatabaseTest, TestGroup, TestGroupSetting, CONTENT, GROUPS,
};
use collab::core::any_map::AnyMapExtension;
use collab_database::views::{CreateViewParams, DatabaseLayout};

#[test]
fn create_database_view_with_group_test() {
  let database_test = create_database_with_two_groups();
  let view = database_test.views.get_view("v1").unwrap();
  assert_eq!(view.group_settings.len(), 2);
  let group_settings = view
    .group_settings
    .iter()
    .map(TestGroupSetting::from)
    .collect::<Vec<TestGroupSetting>>();

  assert_eq!(group_settings[1].id, "g2");
  assert_eq!(group_settings[0].id, "g1");
  assert_eq!(group_settings[0].groups.len(), 2);
  assert_eq!(group_settings[0].groups[0].id, "group_item1");
  assert_eq!(group_settings[0].groups[1].id, "group_item2");
}

#[test]
fn create_database_view_with_group_test2() {
  let database_test = create_database_with_default_data(1, "1");
  let params = CreateViewParams {
    view_id: "v1".to_string(),
    ..Default::default()
  };
  database_test.create_view(params);
  let group_setting = TestGroupSetting {
    id: "g1".to_string(),
    field_id: "".to_string(),
    field_type: Default::default(),
    groups: vec![
      TestGroup {
        id: "group_item1".to_string(),
        name: "group item 1".to_string(),
        visible: false,
      },
      TestGroup {
        id: "group_item2".to_string(),
        name: "group item 2".to_string(),
        visible: false,
      },
    ],
    content: "".to_string(),
  };
  database_test.add_group_setting("v1", group_setting);

  let view = database_test.views.get_view("v1").unwrap();
  assert_eq!(view.group_settings.len(), 1);
  let group_settings = view
    .group_settings
    .iter()
    .map(TestGroupSetting::from)
    .collect::<Vec<TestGroupSetting>>();

  assert_eq!(group_settings[0].id, "g1");
  assert_eq!(group_settings[0].groups.len(), 2);
  assert_eq!(group_settings[0].groups[0].id, "group_item1");
  assert_eq!(group_settings[0].groups[1].id, "group_item2");
}

#[test]
fn extend_database_view_group_test() {
  let database_test = create_database_with_two_groups();
  database_test.update_group_setting("v1", "g1", |object| {
    object.insert_str_value(CONTENT, "hello world".to_string());
    object.extend_with_array(
      GROUPS,
      vec![TestGroup {
        id: "group_item3".to_string(),
        name: "group item 3".to_string(),
        visible: false,
      }],
    );
  });

  let view = database_test.views.get_view("v1").unwrap();
  assert_eq!(view.group_settings.len(), 2);
  let group_settings = view
    .group_settings
    .iter()
    .map(TestGroupSetting::from)
    .collect::<Vec<TestGroupSetting>>();

  assert_eq!(group_settings[0].content, "hello world");
  assert_eq!(group_settings[0].groups.len(), 3);
  assert_eq!(group_settings[0].groups[0].id, "group_item1");
  assert_eq!(group_settings[0].groups[1].id, "group_item2");
  assert_eq!(group_settings[0].groups[2].id, "group_item3");
}

#[test]
fn remove_database_view_group_test() {
  let database_test = create_database_with_two_groups();
  database_test.update_group_setting("v1", "g1", |object| {
    object.remove_array_element(GROUPS, vec!["group_item1"].as_slice());
  });

  let view = database_test.views.get_view("v1").unwrap();
  let group_settings = view
    .group_settings
    .iter()
    .map(TestGroupSetting::from)
    .collect::<Vec<TestGroupSetting>>();

  assert_eq!(group_settings[0].id, "g1");
  assert_eq!(group_settings[0].groups.len(), 1);
  assert_eq!(group_settings[0].groups[0].id, "group_item2");
}

#[test]
fn update_database_view_group_test() {
  let database_test = create_database_with_two_groups();
  let view = database_test.views.get_view("v1").unwrap();
  let group_settings = view
    .group_settings
    .iter()
    .map(TestGroupSetting::from)
    .collect::<Vec<TestGroupSetting>>();
  assert!(!group_settings[0].groups[0].visible);

  database_test.update_group_setting("v1", "g1", |object| {
    object.mut_array_element_by_id(GROUPS, "group_item1", |map| {
      map.insert_bool_value("visible", true);
    });
  });

  let view = database_test.views.get_view("v1").unwrap();
  let group_settings = view
    .group_settings
    .iter()
    .map(TestGroupSetting::from)
    .collect::<Vec<TestGroupSetting>>();
  assert!(group_settings[0].groups[0].visible);
}

fn create_database_with_two_groups() -> DatabaseTest {
  let database_test = create_database_with_default_data(1, "1");
  let group_1 = TestGroupSetting {
    id: "g1".to_string(),
    field_id: "".to_string(),
    field_type: Default::default(),
    groups: vec![
      TestGroup {
        id: "group_item1".to_string(),
        name: "group item 1".to_string(),
        visible: false,
      },
      TestGroup {
        id: "group_item2".to_string(),
        name: "group item 2".to_string(),
        visible: false,
      },
    ],
    content: "".to_string(),
  };
  let group_2 = TestGroupSetting {
    id: "g2".to_string(),
    field_id: "".to_string(),
    field_type: Default::default(),
    groups: vec![],
    content: "".to_string(),
  };

  let params = CreateViewParams {
    view_id: "v1".to_string(),
    groups: vec![group_1.into(), group_2.into()],
    layout: DatabaseLayout::Grid,
    ..Default::default()
  };
  database_test.create_view(params);
  database_test
}