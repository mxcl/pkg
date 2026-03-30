use super::*;

pub fn main_entry() {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_else(|| OsString::from("subs"));
    let invocation = Invocation::from_program(&program);

    let result = match invocation.mode {
        Some(mode) => run_mode(mode, &invocation, args),
        None => dispatch_pkg(&invocation, args),
    };

    if let Err(err) = result {
        eprintln!("{}: {err}", invocation.name);
        process::exit(1);
    }
}

impl Invocation {
    pub(crate) fn from_program(program: &OsString) -> Self {
        let binary_name = Path::new(program)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("subs")
            .to_string();
        let mode = Mode::from_name(&binary_name);
        Self {
            binary_name: binary_name.clone(),
            name: binary_name,
            mode,
        }
    }

    pub(crate) fn for_subcommand(binary_name: &str, subcommand: &str, mode: Mode) -> Self {
        Self {
            binary_name: binary_name.to_string(),
            name: format!("{binary_name} {subcommand}"),
            mode: Some(mode),
        }
    }
}

impl Mode {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "x" | "run" | "use" => Some(Self::X),
            "i" | "install" => Some(Self::I),
            _ => None,
        }
    }

    pub(crate) fn canonical_name(self) -> &'static str {
        match self {
            Self::X => "run",
            Self::I => "install",
        }
    }
}

pub(crate) fn run_mode(
    mode: Mode,
    invocation: &Invocation,
    args: env::ArgsOs,
) -> Result<(), String> {
    match mode {
        Mode::X => run_x(invocation, args),
        Mode::I => run_i(invocation, args),
    }
}

pub(crate) fn run_uninstall(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_uninstall_request(invocation, &mut args)? {
        Some(request) => request,
        None => return Ok(()),
    };

    if !is_root() {
        return Err("must be run as root".to_string());
    }

    let _lock = acquire_package_mutation_lock()?;
    for package in &request.packages {
        ensure_package_installed(Path::new(OPT_PKG_ROOT), package)?;
    }

    for package in request.packages {
        uninstall_package(&package)?;
    }
    Ok(())
}

pub(crate) fn run_outdated(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_package_status_request(invocation, &mut args, print_outdated_usage)? {
        Some(request) => request,
        None => return Ok(()),
    };

    let config = load_config()?;
    for package in resolve_outdated_package_statuses(&config, &request.selection)? {
        println!(
            "{} {} -> {}",
            package.package_name, package.installed_version, package.latest_version
        );
    }
    Ok(())
}

pub(crate) fn run_update(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_update_request(invocation, &mut args)? {
        Some(request) => request,
        None => return Ok(()),
    };

    if !is_root() {
        return Err("must be run as root".to_string());
    }

    maybe_self_update_and_restart(&request)?;

    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    for package in resolve_outdated_package_statuses(&config, &request.selection)? {
        run_i_package(&config, requested_package_from_status(&package), true)?;
    }
    Ok(())
}

pub(crate) fn run_list(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_package_status_request(invocation, &mut args, print_list_usage)? {
        Some(request) => request,
        None => return Ok(()),
    };

    let config = load_config()?;
    for package in resolve_package_statuses(&config, &request.selection)? {
        println!("{} {}", package.package_name, package.installed_version);
    }
    Ok(())
}

pub(crate) fn run_info(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_info_request(invocation, &mut args)? {
        Some(request) => request,
        None => return Ok(()),
    };

    let config = load_config()?;
    print!(
        "{}\n",
        format_package_info(&resolve_package_info(&config, &request.package)?)
    );
    Ok(())
}

pub(crate) fn dispatch_pkg(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let Some(first_arg) = args.next() else {
        print_pkg_usage(&invocation.name);
        return Err("missing subcommand".to_string());
    };

    if is_help_flag(&first_arg) {
        print_pkg_usage(&invocation.name);
        return Ok(());
    }

    if is_version_flag(&first_arg) {
        println!("{PKG_DISPLAY_NAME} {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if matches!(first_arg.to_str(), Some("help")) {
        if let Some(topic) = args.next() {
            match topic.to_str() {
                Some(subcommand) if is_uninstall_subcommand(subcommand) => {
                    print_uninstall_usage(&format!("{} {}", invocation.binary_name, subcommand));
                }
                Some(subcommand) if is_outdated_subcommand(subcommand) => {
                    print_outdated_usage(&format!("{} {}", invocation.binary_name, subcommand));
                }
                Some(subcommand) if is_update_subcommand(subcommand) => {
                    print_update_usage(&format!("{} {}", invocation.binary_name, subcommand));
                }
                Some(subcommand) if is_list_subcommand(subcommand) => {
                    print_list_usage(&format!("{} {}", invocation.binary_name, subcommand));
                }
                Some(subcommand) if is_info_subcommand(subcommand) => {
                    print_info_usage(&format!("{} {}", invocation.binary_name, subcommand));
                }
                Some(subcommand) => match Mode::from_name(subcommand) {
                    Some(mode) => {
                        let nested = Invocation::for_subcommand(
                            &invocation.binary_name,
                            mode.canonical_name(),
                            mode,
                        );
                        print_mode_usage(mode, &nested.name);
                    }
                    None => print_pkg_usage(&invocation.name),
                },
                None => print_pkg_usage(&invocation.name),
            }
        } else {
            print_pkg_usage(&invocation.name);
        }
        return Ok(());
    }

    let subcommand = first_arg
        .to_str()
        .ok_or_else(|| "subcommand must be valid UTF-8".to_string())?;
    if is_uninstall_subcommand(subcommand) {
        return run_uninstall(
            &Invocation {
                binary_name: invocation.binary_name.clone(),
                name: format!("{} {subcommand}", invocation.binary_name),
                mode: None,
            },
            args,
        );
    }
    if is_outdated_subcommand(subcommand) {
        return run_outdated(
            &Invocation {
                binary_name: invocation.binary_name.clone(),
                name: format!("{} {subcommand}", invocation.binary_name),
                mode: None,
            },
            args,
        );
    }
    if is_update_subcommand(subcommand) {
        return run_update(
            &Invocation {
                binary_name: invocation.binary_name.clone(),
                name: format!("{} {subcommand}", invocation.binary_name),
                mode: None,
            },
            args,
        );
    }
    if is_list_subcommand(subcommand) {
        return run_list(
            &Invocation {
                binary_name: invocation.binary_name.clone(),
                name: format!("{} {subcommand}", invocation.binary_name),
                mode: None,
            },
            args,
        );
    }
    if is_info_subcommand(subcommand) {
        return run_info(
            &Invocation {
                binary_name: invocation.binary_name.clone(),
                name: format!("{} {subcommand}", invocation.binary_name),
                mode: None,
            },
            args,
        );
    }
    let Some(mode) = Mode::from_name(subcommand) else {
        print_pkg_usage(&invocation.name);
        return Err(format!("unknown subcommand '{subcommand}'"));
    };
    let nested = Invocation::for_subcommand(&invocation.binary_name, subcommand, mode);
    run_mode(mode, &nested, args)
}

pub(crate) fn parse_x_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
) -> Result<Option<XRequest>, String> {
    parse_x_request_from_iter(invocation, args)
}

pub(crate) fn parse_x_request_from_iter<I>(
    invocation: &Invocation,
    args: I,
) -> Result<Option<XRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let db = load_db()?;
    ensure_db_schema(&db)?;
    parse_x_request_from_iter_with_db(invocation, args, &db)
}

pub(crate) fn parse_x_request_from_iter_with_db<I>(
    invocation: &Invocation,
    mut args: I,
    db: &Db,
) -> Result<Option<XRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let Some(mut first_arg) = args.next() else {
        print_x_usage(&invocation.name);
        return Err("missing executable name".to_string());
    };

    let mut shebang_mode = false;
    if is_shebang_flag(&first_arg) {
        shebang_mode = true;
        first_arg = args.next().ok_or_else(|| {
            print_x_usage(&invocation.name);
            "missing executable name".to_string()
        })?;
    }

    if is_help_flag(&first_arg) {
        print_x_usage(&invocation.name);
        return Ok(None);
    }

    if is_version_flag(&first_arg) {
        println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let mut formulas = Vec::new();
    while let Some(formula) = parse_formula_spec(&first_arg)? {
        push_unique_string(&mut formulas, formula);
        first_arg = args.next().ok_or_else(|| {
            print_x_usage(&invocation.name);
            "missing executable name".to_string()
        })?;
    }

    if formulas.len() > 1 {
        return Err("supports a single root package".to_string());
    }

    let mut tool = first_arg
        .to_str()
        .ok_or_else(|| "tool name must be valid UTF-8".to_string())?
        .to_string();
    if tool.contains('/') {
        return Err("tool name must not contain path separators".to_string());
    }

    if shebang_mode {
        let _ = args.next();
    }
    let tool_args: Vec<OsString> = args.collect();

    let root_package = if let Some(formula) = formulas.pop() {
        XRootPackage::Formula(preferred_run_formula(formula))
    } else if let Some((base_tool, formula)) = split_versioned_tool_token(&tool) {
        tool = base_tool;
        XRootPackage::Formula(preferred_run_formula(formula))
    } else if let Some(package) = vendor_package_for_tool(&tool) {
        XRootPackage::Vendor(package.name.to_string())
    } else {
        XRootPackage::Formula(preferred_run_formula(
            db.entries
                .get(&tool)
                .cloned()
                .ok_or_else(|| format!("no Homebrew formula found for '{tool}'"))?,
        ))
    };

    Ok(Some(XRequest {
        tool,
        tool_args,
        root_package,
    }))
}

pub(crate) fn parse_i_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
) -> Result<Option<IRequest>, String> {
    parse_i_request_from_iter(invocation, args)
}

pub(crate) fn parse_uninstall_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
) -> Result<Option<UninstallRequest>, String> {
    parse_uninstall_request_from_iter(invocation, args)
}

pub(crate) fn parse_update_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
) -> Result<Option<UpdateRequest>, String> {
    parse_update_request_from_iter(invocation, args)
}

pub(crate) fn parse_info_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
) -> Result<Option<InfoRequest>, String> {
    parse_info_request_from_iter(invocation, args)
}

pub(crate) fn parse_package_status_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
    print_usage: fn(&str),
) -> Result<Option<PackageStatusRequest>, String> {
    parse_package_status_request_from_iter(invocation, args, print_usage)
}

pub(crate) fn parse_i_request_from_iter<I>(
    invocation: &Invocation,
    args: I,
) -> Result<Option<IRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let mut force = false;
    let mut packages = Vec::new();

    for arg in args {
        if is_help_flag(&arg) {
            print_i_usage(&invocation.name);
            return Ok(None);
        }

        if is_version_flag(&arg) {
            println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }

        if is_force_flag(&arg) {
            force = true;
            continue;
        }

        packages.push(parse_package_name(&arg)?);
    }

    if packages.is_empty() {
        print_i_usage(&invocation.name);
        return Err("missing package name".to_string());
    }

    Ok(Some(IRequest { packages, force }))
}

pub(crate) fn parse_uninstall_request_from_iter<I>(
    invocation: &Invocation,
    mut args: I,
) -> Result<Option<UninstallRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let Some(first_arg) = args.next() else {
        print_uninstall_usage(&invocation.name);
        return Err("missing package name".to_string());
    };

    if is_help_flag(&first_arg) {
        print_uninstall_usage(&invocation.name);
        return Ok(None);
    }

    if is_version_flag(&first_arg) {
        println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let mut packages = vec![parse_uninstall_package_name(&first_arg)?];
    for arg in args {
        packages.push(parse_uninstall_package_name(&arg)?);
    }

    Ok(Some(UninstallRequest { packages }))
}

pub(crate) fn parse_update_request_from_iter<I>(
    invocation: &Invocation,
    args: I,
) -> Result<Option<UpdateRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let mut no_self_update = false;
    let mut packages = Vec::new();

    for arg in args {
        if is_help_flag(&arg) {
            print_update_usage(&invocation.name);
            return Ok(None);
        }

        if is_version_flag(&arg) {
            println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }

        if is_no_self_update_flag(&arg) {
            no_self_update = true;
            continue;
        }

        packages.push(parse_package_name(&arg)?);
    }

    let selection = if packages.is_empty() {
        PackageSelection::AllInstalled
    } else {
        PackageSelection::Requested(packages)
    };

    Ok(Some(UpdateRequest {
        selection,
        no_self_update,
    }))
}

pub(crate) fn parse_info_request_from_iter<I>(
    invocation: &Invocation,
    mut args: I,
) -> Result<Option<InfoRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let Some(first_arg) = args.next() else {
        print_info_usage(&invocation.name);
        return Err("missing package name".to_string());
    };

    if is_help_flag(&first_arg) {
        print_info_usage(&invocation.name);
        return Ok(None);
    }

    if is_version_flag(&first_arg) {
        println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let package = parse_package_name(&first_arg)?;
    if args.next().is_some() {
        return Err("supports a single package".to_string());
    }

    Ok(Some(InfoRequest { package }))
}

pub(crate) fn parse_package_status_request_from_iter<I>(
    invocation: &Invocation,
    mut args: I,
    print_usage: fn(&str),
) -> Result<Option<PackageStatusRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let Some(first_arg) = args.next() else {
        return Ok(Some(PackageStatusRequest {
            selection: PackageSelection::AllInstalled,
        }));
    };

    if is_help_flag(&first_arg) {
        print_usage(&invocation.name);
        return Ok(None);
    }

    if is_version_flag(&first_arg) {
        println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let mut packages = vec![parse_package_name(&first_arg)?];
    for arg in args {
        packages.push(parse_package_name(&arg)?);
    }

    Ok(Some(PackageStatusRequest {
        selection: PackageSelection::Requested(packages),
    }))
}

pub(crate) fn parse_package_name(value: &OsString) -> Result<RequestedPackage, String> {
    let package = value
        .to_str()
        .ok_or_else(|| "package name must be valid UTF-8".to_string())?
        .to_string();
    if let Some(formula) = package.strip_prefix(BREW_PACKAGE_PREFIX) {
        if formula.is_empty() {
            return Err(format!(
                "package qualifier '{BREW_PACKAGE_PREFIX}' is missing a formula name"
            ));
        }
        if formula.contains('/') {
            return Err(
                "qualified package name must not contain additional path separators".to_string(),
            );
        }
        return Ok(RequestedPackage::HomebrewFormula(formula.to_string()));
    }
    if let Some(npm_package) = package.strip_prefix("npm:") {
        validate_npm_package_name(npm_package)?;
        return Ok(RequestedPackage::NpmPackage(npm_package.to_string()));
    }
    if let Some(pip_package) = package.strip_prefix("pip:") {
        validate_pip_package_name(pip_package)?;
        return Ok(RequestedPackage::PipPackage(normalize_pip_package_name(
            pip_package,
        )));
    }
    if package.contains('/') {
        return Err("package name must not contain path separators".to_string());
    }
    if vendor::get(&package).is_none() {
        if let Some(target) = package_alias_target(&package) {
            return Ok(RequestedPackage::Alias {
                alias: package,
                target: target.clone(),
            });
        }
    }
    Ok(RequestedPackage::Auto(package))
}

pub(crate) fn parse_uninstall_package_name(value: &OsString) -> Result<String, String> {
    let package = value
        .to_str()
        .ok_or_else(|| "package name must be valid UTF-8".to_string())?;
    if let Some(formula) = package.strip_prefix(BREW_PACKAGE_PREFIX) {
        if formula.is_empty() {
            return Err(format!(
                "package qualifier '{BREW_PACKAGE_PREFIX}' is missing a formula name"
            ));
        }
        if formula.contains('/') {
            return Err(
                "qualified package name must not contain additional path separators".to_string(),
            );
        }
        return Ok(formula.to_string());
    }
    if let Some(npm_package) = package.strip_prefix("npm:") {
        validate_npm_package_name(npm_package)?;
        return Ok(npm_package_display_name(npm_package));
    }
    if let Some(pip_package) = package.strip_prefix("pip:") {
        validate_pip_package_name(pip_package)?;
        return Ok(pip_package_display_name(&normalize_pip_package_name(
            pip_package,
        )));
    }
    if package.contains('/') {
        return Err("package name must not contain path separators".to_string());
    }
    if vendor::get(package).is_none() {
        if let Some(target) = package_alias_target(package) {
            return Ok(target.display_name());
        }
    }
    Ok(package.to_string())
}

pub(crate) fn validate_npm_package_name(package: &str) -> Result<(), String> {
    if package.is_empty() {
        return Err("package qualifier 'npm:' is missing a package name".to_string());
    }
    if let Some(scoped) = package.strip_prefix('@') {
        let Some((scope, name)) = scoped.split_once('/') else {
            return Err("scoped npm package names must be in the form @scope/name".to_string());
        };
        if scope.is_empty() || name.is_empty() || name.contains('/') {
            return Err("scoped npm package names must be in the form @scope/name".to_string());
        }
        return Ok(());
    }
    if package.contains('/') {
        return Err("npm package names must not contain path separators".to_string());
    }
    Ok(())
}

pub(crate) fn npm_package_display_name(package: &str) -> String {
    format!("npm:{package}")
}

pub(crate) fn npm_package_install_relative_path(package: &str) -> PathBuf {
    if let Some(scoped) = package.strip_prefix('@') {
        if let Some((scope, name)) = scoped.split_once('/') {
            return PathBuf::from(format!("@{scope}")).join(name);
        }
    }
    PathBuf::from(package)
}

pub(crate) fn npm_package_install_leaf_name(package: &str) -> String {
    package.rsplit('/').next().unwrap_or(package).to_string()
}

pub(crate) fn npm_package_executable_name(package: &str) -> String {
    npm_package_install_leaf_name(package)
}

pub(crate) fn validate_pip_package_name(package: &str) -> Result<(), String> {
    if package.is_empty() {
        return Err("package qualifier 'pip:' is missing a package name".to_string());
    }
    if package.contains('/') {
        return Err("pip package names must not contain path separators".to_string());
    }
    if !package
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(
            "pip package names may only contain ASCII letters, numbers, '.', '-' and '_'"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn normalize_pip_package_name(package: &str) -> String {
    let mut normalized = String::with_capacity(package.len());
    let mut saw_separator = false;
    for ch in package.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            normalized.push(lower);
            saw_separator = false;
        } else if matches!(lower, '-' | '_' | '.') && !saw_separator {
            normalized.push('-');
            saw_separator = true;
        }
    }
    normalized.trim_matches('-').to_string()
}

pub(crate) fn pip_package_display_name(package: &str) -> String {
    format!("pip:{package}")
}

pub(crate) fn pip_package_install_leaf_name(package: &str) -> String {
    normalize_pip_package_name(package)
}

impl PackageAliasTarget {
    pub(crate) fn display_name(&self) -> String {
        match self {
            Self::HomebrewFormula(formula) => format!("{BREW_PACKAGE_PREFIX}{formula}"),
            Self::NpmPackage(package) => npm_package_display_name(package),
            Self::PipPackage(package) => pip_package_display_name(package),
        }
    }

    pub(crate) fn into_requested_package(self) -> RequestedPackage {
        match self {
            Self::HomebrewFormula(formula) => RequestedPackage::HomebrewFormula(formula),
            Self::NpmPackage(package) => RequestedPackage::NpmPackage(package),
            Self::PipPackage(package) => RequestedPackage::PipPackage(package),
        }
    }
}

pub(crate) fn embedded_package_aliases() -> &'static HashMap<String, PackageAliasTarget> {
    PACKAGE_ALIASES.get_or_init(|| {
        serde_json::from_str::<HashMap<String, String>>(EMBEDDED_ALIASES)
            .expect("failed to parse embedded package aliases JSON")
            .into_iter()
            .map(|(alias, target)| {
                let parsed = parse_package_alias_target(&target)
                    .unwrap_or_else(|err| panic!("invalid alias target {target}: {err}"));
                (alias, parsed)
            })
            .collect()
    })
}

pub(crate) fn package_alias_target(package: &str) -> Option<&'static PackageAliasTarget> {
    embedded_package_aliases().get(package)
}

pub(crate) fn parse_package_alias_target(value: &str) -> Result<PackageAliasTarget, String> {
    if let Some(formula) = value.strip_prefix(BREW_PACKAGE_PREFIX) {
        if formula.is_empty() {
            return Err(format!(
                "package qualifier '{BREW_PACKAGE_PREFIX}' is missing a formula name"
            ));
        }
        if formula.contains('/') {
            return Err(
                "qualified package name must not contain additional path separators".to_string(),
            );
        }
        return Ok(PackageAliasTarget::HomebrewFormula(formula.to_string()));
    }
    if let Some(npm_package) = value.strip_prefix("npm:") {
        validate_npm_package_name(npm_package)?;
        return Ok(PackageAliasTarget::NpmPackage(npm_package.to_string()));
    }
    if let Some(pip_package) = value.strip_prefix("pip:") {
        validate_pip_package_name(pip_package)?;
        return Ok(PackageAliasTarget::PipPackage(normalize_pip_package_name(
            pip_package,
        )));
    }
    Err("alias targets must use a package qualifier".to_string())
}

pub(crate) fn package_install_root(opt_root: &Path, package_name: &str) -> Result<PathBuf, String> {
    if let Some(npm_package) = package_name.strip_prefix("npm:") {
        validate_npm_package_name(npm_package)?;
        return Ok(opt_root
            .join("npm")
            .join(npm_package_install_relative_path(npm_package)));
    }
    if let Some(pip_package) = package_name.strip_prefix("pip:") {
        validate_pip_package_name(pip_package)?;
        return Ok(opt_root
            .join("pip")
            .join(pip_package_install_leaf_name(pip_package)));
    }
    Ok(opt_root.join(package_name))
}

pub(crate) fn resolve_i_root_formula(package: &str) -> Result<String, String> {
    let db = load_db()?;
    ensure_db_schema(&db)?;
    resolve_i_root_formula_with_db(package, &db, formula_metadata_exists)
}

pub(crate) fn ensure_alias_install_target_unambiguous(
    alias: &str,
    target: &PackageAliasTarget,
) -> Result<(), String> {
    let db = load_db()?;
    ensure_db_schema(&db)?;
    ensure_alias_install_target_unambiguous_with_db(alias, target, &db, formula_metadata_exists)
}

pub(crate) fn resolve_i_root_formula_with_db<F>(
    package: &str,
    db: &Db,
    formula_exists: F,
) -> Result<String, String>
where
    F: FnOnce(&str) -> Result<bool, String>,
{
    let Some(formula) = db.entries.get(package) else {
        return Ok(package.to_string());
    };
    if formula == package {
        return Ok(formula.clone());
    }
    if formula_exists(package)? {
        return Err(ambiguous_install_target_message(package, formula));
    }
    Ok(formula.clone())
}

pub(crate) fn ensure_alias_install_target_unambiguous_with_db<F>(
    alias: &str,
    target: &PackageAliasTarget,
    db: &Db,
    formula_exists: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<bool, String>,
{
    if let Some(formula) = db.entries.get(alias) {
        if formula == alias {
            return Err(ambiguous_alias_formula_message(alias, target));
        }
        return Err(ambiguous_alias_executable_message(alias, formula, target));
    }
    if formula_exists(alias)? {
        return Err(ambiguous_alias_formula_message(alias, target));
    }
    Ok(())
}

pub(crate) fn recommended_full_formula(formula: &str) -> Option<&'static str> {
    match formula {
        "ffmpeg" => Some("ffmpeg-full"),
        "imagemagick" => Some("imagemagick-full"),
        _ => None,
    }
}

pub(crate) fn preferred_run_formula(formula: String) -> String {
    recommended_full_formula(&formula)
        .map(str::to_string)
        .unwrap_or(formula)
}

pub(crate) fn vendor_package_for_tool(tool: &str) -> Option<vendor::VendorPackage> {
    vendor::PACKAGES
        .iter()
        .copied()
        .find(|entry| (entry.executables)().contains(&tool))
        .map(vendor::VendorEntry::package)
}

pub(crate) fn print_full_formula_recommendation(formula: &str) -> Result<(), String> {
    let mut stderr = std::io::stderr();
    write_full_formula_recommendation(formula, &mut stderr)
}

pub(crate) fn write_full_formula_recommendation<W: Write>(
    formula: &str,
    stderr: &mut W,
) -> Result<(), String> {
    let Some(recommended) = recommended_full_formula(formula) else {
        return Ok(());
    };
    writeln!(
        stderr,
        "info: requested `{formula}`; `{BREW_PACKAGE_PREFIX}{recommended}` is recommended instead"
    )
    .map_err(|err| format!("failed to write stderr: {err}"))
}

pub(crate) fn split_versioned_tool_token(token: &str) -> Option<(String, String)> {
    let (base, version) = token.rsplit_once('@')?;
    if base.is_empty() || version.is_empty() || !version.chars().any(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((base.to_string(), token.to_string()))
}

pub(crate) fn load_db() -> Result<Db, String> {
    serde_json::from_slice(EMBEDDED_DB).map_err(|err| format!("failed to parse embedded db: {err}"))
}

pub(crate) fn ensure_db_schema(db: &Db) -> Result<(), String> {
    if db.schema != DB_SCHEMA_VERSION {
        return Err(format!(
            "unsupported db schema {} (expected {})",
            db.schema, DB_SCHEMA_VERSION
        ));
    }
    Ok(())
}

pub(crate) fn print_x_usage(program: &str) {
    println!("Usage: {program} [-! | --shebang] [+formula] <executable> [args...]");
    println!();
    println!("Runs Homebrew executables ephemerally from a fresh temp dir.");
}

pub(crate) fn print_i_usage(program: &str) {
    println!("Usage: {program} [-f | --force] <package|brew:formula|npm:package|pip:package>...");
    println!();
    println!("Installs self-contained packages under {OPT_PKG_ROOT}.");
}

pub(crate) fn print_uninstall_usage(program: &str) {
    println!("Usage: {program} <package|brew:formula|npm:package|pip:package>...");
    println!();
    println!("Removes installed packages from {OPT_PKG_ROOT}.");
}

pub(crate) fn print_outdated_usage(program: &str) {
    println!("Usage: {program} [package|brew:formula|npm:package|pip:package]...");
    println!();
    println!("Lists installed packages with newer versions available.");
}

pub(crate) fn print_update_usage(program: &str) {
    println!("Usage: {program} [package|brew:formula|npm:package|pip:package]...");
    println!();
    println!("Reinstalls installed packages with newer versions available.");
}

pub(crate) fn print_list_usage(program: &str) {
    println!("Usage: {program} [package|brew:formula|npm:package|pip:package]...");
    println!();
    println!("Lists installed packages with their versions.");
}

pub(crate) fn print_info_usage(program: &str) {
    println!("Usage: {program} <package|brew:formula|npm:package|pip:package>");
    println!();
    println!("Shows package metadata, install status, and update status.");
}

pub(crate) fn print_pkg_usage(program: &str) {
    println!("Usage: {program} <subcommand> [args...]");
    println!();
    println!("Subcommands:");
    println!("  run, x         Run Homebrew executables ephemerally.");
    println!("  install, i     Install a self-contained package.");
    println!("  info           Show package metadata and status.");
    println!("  list, ls       List installed packages with their versions.");
    println!("  outdated       List installed packages with updates available.");
    println!("  update, up     Reinstall installed packages with updates available.");
    println!("  uninstall, rm  Remove an installed package.");
}

pub(crate) fn print_mode_usage(mode: Mode, program: &str) {
    match mode {
        Mode::X => print_x_usage(program),
        Mode::I => print_i_usage(program),
    }
}

pub(crate) fn is_help_flag(value: &OsString) -> bool {
    matches!(value.to_str(), Some("-h" | "--help"))
}

pub(crate) fn is_version_flag(value: &OsString) -> bool {
    matches!(value.to_str(), Some("-V" | "--version"))
}

pub(crate) fn is_force_flag(value: &OsString) -> bool {
    matches!(value.to_str(), Some("-f" | "--force"))
}

pub(crate) fn is_shebang_flag(value: &OsString) -> bool {
    matches!(value.to_str(), Some("-!" | "--shebang"))
}

pub(crate) fn is_no_self_update_flag(value: &OsString) -> bool {
    matches!(value.to_str(), Some(SELF_UPDATE_DISABLE_FLAG))
}

pub(crate) fn is_uninstall_subcommand(value: &str) -> bool {
    matches!(value, "uninstall" | "rm")
}

pub(crate) fn is_outdated_subcommand(value: &str) -> bool {
    value == "outdated"
}

pub(crate) fn is_update_subcommand(value: &str) -> bool {
    matches!(value, "update" | "up")
}

pub(crate) fn is_list_subcommand(value: &str) -> bool {
    matches!(value, "list" | "ls")
}

pub(crate) fn is_info_subcommand(value: &str) -> bool {
    value == "info"
}

pub(crate) fn parse_formula_spec(value: &OsString) -> Result<Option<String>, String> {
    let Some(value) = value.to_str() else {
        return Ok(None);
    };
    let Some(formula) = value.strip_prefix('+') else {
        return Ok(None);
    };
    if formula.is_empty() {
        return Err("package spec '+' is missing a formula name".to_string());
    }
    if formula.contains('/') {
        return Err(format!(
            "package spec '+{formula}' must not contain path separators"
        ));
    }
    Ok(Some(formula.to_string()))
}
