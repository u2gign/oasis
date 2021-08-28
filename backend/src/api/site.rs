use crate::entity::file::File;
use crate::entity::user::User;
use crate::service::state::State;
use crate::util::init;
use crate::{request::site::SiteSetupRequest, util};
use sqlx::Acquire;
use tide::{Request, Response, Result, StatusCode};
use util::file_system;

// Post "/api/setup"
pub async fn post_setup(mut req: Request<State>) -> Result {
    let first_run = req.state().get_first_run()?;
    if !first_run {
        return Ok(Response::new(StatusCode::Unauthorized));
    }

    let mut setup_req = SiteSetupRequest::from_req(&mut req).await?;
    if !setup_req.validate()? {
        return Ok(Response::new(StatusCode::BadRequest));
    }
    let mut conn = req.state().get_pool_conn().await?;
    if User::find_exist_username(&setup_req.username, &mut conn)
        .await?
        .is_some()
    {
        return Ok(Response::new(StatusCode::Conflict));
    }

    let root_path = init::create_site_dir(&setup_req.storage).await?;
    setup_req.storage = root_path.to_string_lossy().to_string();
    file_system::create_user_dirs(&setup_req.storage, &setup_req.username).await?;
    let secret = util::generate_secret_key();
    let site = setup_req.to_site(&secret);
    let mut user = setup_req.to_admin();

    let mut tx = conn.begin().await?;
    site.create_query(&mut tx).await?;
    let user_id = user.create_query(&mut tx).await?;
    user.user_id = user_id;
    user.create_root_file().create_query(&mut tx).await?;
    tx.commit().await?;

    req.state().set_site(site)?;

    Ok(Response::new(200))
}
