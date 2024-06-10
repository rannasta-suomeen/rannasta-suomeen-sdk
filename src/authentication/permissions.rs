use crate::{jwt::SessionData, schema::UserRole};

const ACTION_TABLE: &[(UserRole, &[ActionType])] = &[
    (UserRole::User, &[ActionType::ManageOwnFavorites]),
    (
        UserRole::Creator,
        &[
            ActionType::ManageOwnFavorites,
            ActionType::CreateIncredients,
            ActionType::CreateRecipes,
            ActionType::ManageOwnRecipes,
            ActionType::ManageOwnIncredients,
        ],
    ),
    (
        UserRole::Admin,
        &[
            ActionType::ManageOwnFavorites,
            ActionType::CreateIncredients,
            ActionType::CreateRecipes,
            ActionType::ManageOwnRecipes,
            ActionType::ManageOwnIncredients,
            ActionType::ManageAllRecipes,
            ActionType::ManageAllIncredients,
        ],
    ),
];

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionType {
    CreateRecipes,
    CreateIncredients,

    ManageOwnFavorites,
    ManageOwnRecipes,
    ManageOwnIncredients,

    ManageUsers,
    ManageAllRecipes,
    ManageAllIncredients,
}

impl ActionType {
    pub fn authenticate(self, session: &SessionData) -> bool {
        let user_uid = &session.user_uid;

        ACTION_TABLE
            .iter()
            .find_map(|(uid, actions)| {
                if user_uid != uid {
                    return None;
                }

                Some(actions.contains(&self))
            })
            .unwrap()
    }
}
