use spacetimedb::ReducerContext;
use spacetimedsl::dsl;


pub fn try_server_or_dev(ctx: &ReducerContext) -> bool {
    try_developer_only(ctx) || try_server_only(ctx)
}

pub fn try_developer_only(ctx: &ReducerContext) -> bool {
    if ctx.sender.to_string().contains("c200a78183f5f9062ea") {
        log::trace!("Developer user {} is performing a developer-only action", ctx.sender);
        return true;
    }
    else {
        log::warn!("SECURITY: Non-developer user {} attempted developer-only action", ctx.sender);
        return false;
    }
}

pub fn try_server_only(ctx: &ReducerContext) -> bool {
    if ctx.sender == ctx.identity() {
        return true;
    }
    else {
        log::warn!("SECURITY: Non-server user {} attempted server-only action", ctx.sender);
        return false;
    }
}

pub fn create_initial_records(ctx: &ReducerContext) -> Result<(), String> {
    let _dsl = dsl(ctx);
    if !try_server_only(ctx) {
        return Err("Unauthorized access".to_string());
    }
    Ok(())
}

