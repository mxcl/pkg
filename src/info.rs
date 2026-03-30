use super::*;

pub(crate) const INFO_WIDTH: usize = 64;
pub(crate) const INFO_INNER_WIDTH: usize = INFO_WIDTH - 2;
pub(crate) const INFO_LABEL_WIDTH: usize = 14;

pub(crate) fn load_config() -> Result<Config, String> {
    let bottle_tag = current_bottle_tag()?;
    Ok(Config { bottle_tag })
}

impl PackageStatus {
    pub(crate) fn is_outdated(&self) -> bool {
        self.installed_version != self.latest_version
    }
}

pub(crate) fn resolve_package_statuses(
    config: &Config,
    selection: &PackageSelection,
) -> Result<Vec<PackageStatus>, String> {
    match selection {
        PackageSelection::AllInstalled => resolve_scanned_package_statuses(
            installed_package_refs(Path::new(OPT_PKG_ROOT))?,
            |package| {
                resolve_package_status_at(config, &package.package_name, &package.install_root)
            },
            |message| eprintln!("{message}"),
        ),
        PackageSelection::Requested(packages) => {
            let mut package_names = packages
                .iter()
                .map(requested_package_name)
                .collect::<Vec<_>>();
            package_names.sort();
            package_names.dedup();

            let mut statuses = Vec::with_capacity(package_names.len());
            for package_name in package_names {
                statuses.push(resolve_package_status(config, &package_name)?);
            }
            Ok(statuses)
        }
    }
}

pub(crate) fn resolve_outdated_package_statuses(
    config: &Config,
    selection: &PackageSelection,
) -> Result<Vec<PackageStatus>, String> {
    Ok(filter_outdated_package_statuses(resolve_package_statuses(
        config, selection,
    )?))
}

pub(crate) fn filter_outdated_package_statuses(statuses: Vec<PackageStatus>) -> Vec<PackageStatus> {
    statuses
        .into_iter()
        .filter(PackageStatus::is_outdated)
        .collect()
}

pub(crate) fn resolve_scanned_package_statuses<Resolve, Warn>(
    mut packages: Vec<InstalledPackageRef>,
    mut resolve: Resolve,
    mut warn: Warn,
) -> Result<Vec<PackageStatus>, String>
where
    Resolve: FnMut(&InstalledPackageRef) -> Result<PackageStatus, String>,
    Warn: FnMut(String),
{
    packages.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    packages.dedup_by(|left, right| left.package_name == right.package_name);

    let mut statuses = Vec::with_capacity(packages.len());
    for package in packages {
        match resolve(&package) {
            Ok(status) => statuses.push(status),
            Err(err) => warn(format!(
                "warning: skipping {}: {err}",
                package.install_root.display()
            )),
        }
    }
    Ok(statuses)
}

pub(crate) fn resolve_package_status(
    config: &Config,
    package_name: &str,
) -> Result<PackageStatus, String> {
    let install_root = package_install_root(Path::new(OPT_PKG_ROOT), package_name)?;
    resolve_package_status_at(config, package_name, &install_root)
}

pub(crate) fn resolve_package_status_at(
    config: &Config,
    package_name: &str,
    install_root: &Path,
) -> Result<PackageStatus, String> {
    let metadata = fs::symlink_metadata(install_root).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => format!("package {package_name} is not installed"),
        _ => format!("failed to stat {}: {err}", install_root.display()),
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "installed package root {} is not a directory",
            install_root.display()
        ));
    }

    let receipt = load_or_resolve_package_receipt(package_name, install_root)?;
    let latest_version = resolve_latest_version_for_source(config, &receipt.source)?;

    Ok(PackageStatus {
        package_name: receipt.package_name,
        source: receipt.source,
        installed_version: receipt.version,
        latest_version,
    })
}

pub(crate) fn requested_package_name(package: &RequestedPackage) -> String {
    match package {
        RequestedPackage::Auto(package_name) | RequestedPackage::HomebrewFormula(package_name) => {
            package_name.clone()
        }
        RequestedPackage::Alias { target, .. } => target.display_name(),
        RequestedPackage::NpmPackage(package_name) => npm_package_display_name(package_name),
        RequestedPackage::PipPackage(package_name) => pip_package_display_name(package_name),
    }
}

pub(crate) fn requested_package_from_status(status: &PackageStatus) -> RequestedPackage {
    match &status.source {
        PackageReceiptSource::Formula { root_formula } if status.package_name == *root_formula => {
            RequestedPackage::HomebrewFormula(root_formula.clone())
        }
        PackageReceiptSource::Npm { package_name } => {
            RequestedPackage::NpmPackage(package_name.clone())
        }
        PackageReceiptSource::Pip { package_name } => {
            RequestedPackage::PipPackage(package_name.clone())
        }
        _ => RequestedPackage::Auto(status.package_name.clone()),
    }
}

pub(crate) fn resolve_package_info(
    config: &Config,
    requested: &RequestedPackage,
) -> Result<PackageInfo, String> {
    let package_name = requested_package_name(requested);
    let install_root = package_install_root(Path::new(OPT_PKG_ROOT), &package_name)?;
    let metadata = match fs::symlink_metadata(&install_root) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("failed to stat {}: {err}", install_root.display())),
    };

    if let Some(metadata) = metadata {
        if !metadata.is_dir() {
            return Err(format!(
                "installed package root {} is not a directory",
                install_root.display()
            ));
        }
        return resolve_installed_package_info(config, requested, package_name, install_root);
    }

    Ok(resolve_uninstalled_package_info(
        config,
        requested,
        package_name,
        install_root,
    ))
}

pub(crate) fn resolve_installed_package_info(
    config: &Config,
    requested: &RequestedPackage,
    package_name: String,
    install_root: PathBuf,
) -> Result<PackageInfo, String> {
    let mut info = PackageInfo {
        package_name,
        qualified_name: String::new(),
        install_root,
        installed: true,
        source: None,
        source_error: None,
        aliases: Vec::new(),
        aliases_error: None,
        installed_version: None,
        latest_version: None,
        latest_version_error: None,
        executable_paths: Vec::new(),
        executable_paths_error: None,
        homebrew_info: None,
        homebrew_info_error: None,
        npm_homepage: None,
        npm_package_info_error: None,
    };

    match load_package_receipt(&info.install_root.join(ROOT_RECEIPT)) {
        Ok(Some(receipt)) => {
            info.package_name = receipt.package_name;
            info.source = Some(receipt.source);
            info.installed_version = Some(receipt.version);
        }
        Ok(None) => info.source_error = Some("missing package metadata".to_string()),
        Err(err) => info.source_error = Some(err),
    }

    if info.source.is_none() {
        info.source = explicit_requested_package_source(requested);
    }
    match installed_stub_paths_at(&info.install_root) {
        Ok(paths) => info.executable_paths = paths,
        Err(err) => info.executable_paths_error = Some(err),
    }
    populate_package_info_identity(&mut info);
    populate_package_info_metadata(config, &mut info);
    Ok(info)
}

pub(crate) fn resolve_uninstalled_package_info(
    config: &Config,
    requested: &RequestedPackage,
    package_name: String,
    install_root: PathBuf,
) -> PackageInfo {
    let mut info = PackageInfo {
        package_name,
        qualified_name: String::new(),
        install_root,
        installed: false,
        source: None,
        source_error: None,
        aliases: Vec::new(),
        aliases_error: None,
        installed_version: None,
        latest_version: None,
        latest_version_error: None,
        executable_paths: Vec::new(),
        executable_paths_error: None,
        homebrew_info: None,
        homebrew_info_error: None,
        npm_homepage: None,
        npm_package_info_error: None,
    };

    match infer_requested_package_source(requested) {
        Ok(source) => info.source = Some(source),
        Err(err) => info.source_error = Some(err),
    }
    populate_package_info_identity(&mut info);
    populate_package_info_metadata(config, &mut info);
    info
}

pub(crate) fn populate_package_info_identity(info: &mut PackageInfo) {
    if let Some(source) = info.source.as_ref() {
        info.qualified_name = package_source_qualified_name(source);
        let (aliases, alias_error) = resolve_aliases_for_source(source);
        info.aliases = aliases;
        info.aliases_error = alias_error;
    } else {
        info.qualified_name = info.package_name.clone();
    }
}

pub(crate) fn populate_package_info_metadata(config: &Config, info: &mut PackageInfo) {
    let Some(source) = info.source.as_ref() else {
        return;
    };

    match source {
        PackageReceiptSource::Formula { root_formula } => match fetch_formula_info(root_formula) {
            Ok(formula_info) => {
                info.homebrew_info = Some(homebrew_package_info_from_formula_info(
                    root_formula,
                    &formula_info,
                ));
                match ensure_formula_has_bottle(root_formula, &formula_info, &config.bottle_tag) {
                    Ok(()) => info.latest_version = Some(formula_version_string(&formula_info)),
                    Err(err) => info.latest_version_error = Some(err),
                }
            }
            Err(err) => {
                info.latest_version_error = Some(err.clone());
                info.homebrew_info_error = Some(err);
            }
        },
        PackageReceiptSource::Npm { package_name } => {
            match resolve_latest_version_for_source(config, source) {
                Ok(latest_version) => info.latest_version = Some(latest_version),
                Err(err) => info.latest_version_error = Some(err),
            }
            match resolve_npm_homepage(package_name) {
                Ok(homepage) => info.npm_homepage = homepage,
                Err(err) => info.npm_package_info_error = Some(err),
            }
        }
        _ => match resolve_latest_version_for_source(config, source) {
            Ok(latest_version) => info.latest_version = Some(latest_version),
            Err(err) => info.latest_version_error = Some(err),
        },
    }
}

pub(crate) fn explicit_requested_package_source(
    requested: &RequestedPackage,
) -> Option<PackageReceiptSource> {
    match requested {
        RequestedPackage::HomebrewFormula(formula) => Some(PackageReceiptSource::Formula {
            root_formula: formula.clone(),
        }),
        RequestedPackage::Alias { target, .. } => match target {
            PackageAliasTarget::HomebrewFormula(formula) => Some(PackageReceiptSource::Formula {
                root_formula: formula.clone(),
            }),
            PackageAliasTarget::NpmPackage(package_name) => Some(PackageReceiptSource::Npm {
                package_name: package_name.clone(),
            }),
            PackageAliasTarget::PipPackage(package_name) => Some(PackageReceiptSource::Pip {
                package_name: package_name.clone(),
            }),
        },
        RequestedPackage::NpmPackage(package_name) => Some(PackageReceiptSource::Npm {
            package_name: package_name.clone(),
        }),
        RequestedPackage::PipPackage(package_name) => Some(PackageReceiptSource::Pip {
            package_name: package_name.clone(),
        }),
        RequestedPackage::Auto(_) => None,
    }
}

pub(crate) fn infer_requested_package_source(
    requested: &RequestedPackage,
) -> Result<PackageReceiptSource, String> {
    if let Some(source) = explicit_requested_package_source(requested) {
        return Ok(source);
    }

    let RequestedPackage::Auto(package_name) = requested else {
        unreachable!("qualified and aliased packages are handled above")
    };
    if let Some(package) = vendor::get(package_name) {
        return Ok(PackageReceiptSource::Vendor {
            vendor_name: package.name.to_string(),
        });
    }

    Ok(PackageReceiptSource::Formula {
        root_formula: resolve_i_root_formula(package_name)?,
    })
}

pub(crate) fn resolve_latest_version_for_source(
    config: &Config,
    source: &PackageReceiptSource,
) -> Result<String, String> {
    match source {
        PackageReceiptSource::Formula { root_formula } => {
            resolve_formula_latest_version(config, root_formula)
        }
        PackageReceiptSource::Vendor { vendor_name } => resolve_vendor_latest_version(vendor_name),
        PackageReceiptSource::Npm { package_name } => resolve_npm_latest_version(package_name),
        PackageReceiptSource::Pip { package_name } => resolve_pip_latest_version(package_name),
    }
}

pub(crate) fn package_source_qualified_name(source: &PackageReceiptSource) -> String {
    match source {
        PackageReceiptSource::Formula { root_formula } => {
            format!("{BREW_PACKAGE_PREFIX}{root_formula}")
        }
        PackageReceiptSource::Vendor { vendor_name } => format!("subs:{vendor_name}"),
        PackageReceiptSource::Npm { package_name } => npm_package_display_name(package_name),
        PackageReceiptSource::Pip { package_name } => pip_package_display_name(package_name),
    }
}

pub(crate) fn resolve_aliases_for_source(
    source: &PackageReceiptSource,
) -> (Vec<String>, Option<String>) {
    let mut aliases = our_aliases_for_source(source);
    let mut alias_error = None;

    if let PackageReceiptSource::Formula { root_formula } = source {
        match homebrew_aliases_for_formula(root_formula) {
            Ok(mut brew_aliases) => aliases.append(&mut brew_aliases),
            Err(err) => alias_error = Some(err),
        }
    }

    aliases.sort();
    aliases.dedup();
    (aliases, alias_error)
}

pub(crate) fn our_aliases_for_source(source: &PackageReceiptSource) -> Vec<String> {
    let qualified_name = package_source_qualified_name(source);
    let mut aliases = embedded_package_aliases()
        .iter()
        .filter_map(|(alias, target)| {
            (target.display_name() == qualified_name).then_some(alias.clone())
        })
        .collect::<Vec<_>>();
    aliases.sort();
    aliases
}

pub(crate) fn homebrew_aliases_for_formula(formula: &str) -> Result<Vec<String>, String> {
    let mut aliases = formula_alias_index()?
        .iter()
        .filter_map(|(alias, canonical)| (canonical == formula).then_some(alias.clone()))
        .collect::<Vec<_>>();
    aliases.sort();
    Ok(aliases)
}

pub(crate) fn homebrew_package_info_from_formula_info(
    formula: &str,
    info: &FormulaInfo,
) -> HomebrewPackageInfo {
    HomebrewPackageInfo {
        formula: formula.to_string(),
        description: string_or_none(&info.desc),
        homepage: string_or_none(&info.homepage),
        license: info
            .license
            .clone()
            .and_then(|value| string_or_none(&value)),
        dependencies: info.dependencies.clone(),
    }
}

pub(crate) fn string_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn format_package_info(info: &PackageInfo) -> String {
    let installed_value = if info.installed {
        info.install_root.display().to_string()
    } else {
        "no".to_string()
    };
    let mut lines = vec![plain_box_top()];
    for (index, line) in wrap_text(&info.qualified_name, INFO_WIDTH - 6)
        .into_iter()
        .enumerate()
    {
        if index == 0 {
            lines.push(format!("   📦 {line}"));
        } else {
            lines.push(format!("     {line}"));
        }
    }
    lines.push(plain_box_bottom());
    lines.push(String::new());

    push_single_line_field(
        &mut lines,
        "Version",
        &format_version_value(info),
        format_version_status(info).as_deref(),
    );
    push_single_line_field(&mut lines, "Installed", &installed_value, None);
    push_wrapped_field(
        &mut lines,
        "Source",
        &format_source_field(info.source.as_ref()),
    );
    if !info.aliases.is_empty() {
        push_wrapped_field(&mut lines, "Aliases", &info.aliases.join(", "));
    }

    let mut metadata_lines = Vec::new();
    if let Some(homebrew_info) = info.homebrew_info.as_ref() {
        if let Some(description) = homebrew_info.description.as_deref() {
            push_wrapped_field(&mut metadata_lines, "Description", description);
        }
        if let Some(homepage) = homebrew_info.homepage.as_deref() {
            push_wrapped_field(&mut metadata_lines, "Homepage", homepage);
        }
        if let Some(license) = homebrew_info.license.as_deref() {
            push_wrapped_field(&mut metadata_lines, "License", license);
        }
        push_wrapped_field(
            &mut metadata_lines,
            "Formula Page",
            &homebrew_formula_page_url(&homebrew_info.formula),
        );
    } else if let Some(PackageReceiptSource::Formula { root_formula }) = info.source.as_ref() {
        push_wrapped_field(
            &mut metadata_lines,
            "Formula Page",
            &homebrew_formula_page_url(root_formula),
        );
        if let Some(err) = info.homebrew_info_error.as_deref() {
            push_wrapped_field(
                &mut metadata_lines,
                "Homebrew Info",
                &format!("unavailable ({err})"),
            );
        }
    }
    if let Some(PackageReceiptSource::Npm { .. }) = info.source.as_ref() {
        if let Some(homepage) = info.npm_homepage.as_deref() {
            push_wrapped_field(&mut metadata_lines, "Homepage", homepage);
        } else if let Some(err) = info.npm_package_info_error.as_deref() {
            push_wrapped_field(
                &mut metadata_lines,
                "Homepage",
                &format!("unavailable ({err})"),
            );
        }
    }

    if !metadata_lines.is_empty() {
        lines.push(String::new());
        lines.extend(metadata_lines);
    }

    if let Some(homebrew_info) = info.homebrew_info.as_ref() {
        if !homebrew_info.dependencies.is_empty() {
            lines.push(String::new());
            lines.push(section_top("Dependencies"));
            for line in wrap_tokens(&homebrew_info.dependencies, 2, 3) {
                lines.push(line);
            }
            lines.push(section_bottom());
        }
    }

    if !info.executable_paths.is_empty() || info.executable_paths_error.is_some() {
        lines.push(String::new());
        lines.push(section_top("Executables"));
        if let Some(err) = info.executable_paths_error.as_deref() {
            for line in wrap_text(&format!("unavailable ({err})"), INFO_INNER_WIDTH - 2) {
                lines.push(format!("  {line}"));
            }
        } else {
            for executable in &info.executable_paths {
                for line in wrap_text(executable, INFO_INNER_WIDTH - 2) {
                    lines.push(format!("  {line}"));
                }
            }
        }
        lines.push(section_bottom());
    }

    lines.join("\n")
}

pub(crate) fn plain_box_top() -> String {
    format!("╭{}╮", "─".repeat(INFO_INNER_WIDTH))
}

pub(crate) fn plain_box_bottom() -> String {
    format!("╰{}╯", "─".repeat(INFO_INNER_WIDTH))
}

pub(crate) fn section_top(title: &str) -> String {
    let prefix = format!("╭─ {title} ");
    let fill = "─".repeat(INFO_WIDTH - prefix.chars().count() - 1);
    format!("{prefix}{fill}╮")
}

pub(crate) fn section_bottom() -> String {
    format!("╰{}╯", "─".repeat(INFO_INNER_WIDTH))
}

pub(crate) fn push_single_line_field(
    lines: &mut Vec<String>,
    label: &str,
    value: &str,
    suffix: Option<&str>,
) {
    let mut line = format!("  {label:<INFO_LABEL_WIDTH$}{value}");
    if let Some(suffix) = suffix {
        line.push_str("  ");
        line.push_str(suffix);
    }
    lines.push(line);
}

pub(crate) fn push_wrapped_field(lines: &mut Vec<String>, label: &str, value: &str) {
    let wrapped = wrap_text(value, INFO_WIDTH - 2 - INFO_LABEL_WIDTH - 2);
    let mut iter = wrapped.into_iter();
    if let Some(first) = iter.next() {
        lines.push(format!("  {label:<INFO_LABEL_WIDTH$}{first}"));
        for line in iter {
            lines.push(format!("  {:<INFO_LABEL_WIDTH$}{line}", ""));
        }
    } else {
        lines.push(format!("  {label:<INFO_LABEL_WIDTH$}"));
    }
}

pub(crate) fn wrap_text(value: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for paragraph in value.lines() {
        if paragraph.is_empty() {
            if lines.is_empty() || !lines.last().unwrap().is_empty() {
                lines.push(String::new());
            }
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let chunks = split_text_hard(word, width);
            for chunk in chunks {
                let next_len = if current.is_empty() {
                    chunk.chars().count()
                } else {
                    current.chars().count() + 1 + chunk.chars().count()
                };
                if !current.is_empty() && next_len > width {
                    lines.push(current);
                    current = chunk;
                } else {
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.push_str(&chunk);
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(crate) fn split_text_hard(value: &str, width: usize) -> Vec<String> {
    if value.chars().count() <= width {
        return vec![value.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

pub(crate) fn wrap_tokens(tokens: &[String], indent: usize, gap: usize) -> Vec<String> {
    let indent_str = " ".repeat(indent);
    let gap_str = " ".repeat(gap);
    let mut lines = Vec::new();
    let mut current = indent_str.clone();
    for token in tokens {
        let candidate = if current.trim().is_empty() {
            format!("{indent_str}{token}")
        } else {
            format!("{current}{gap_str}{token}")
        };
        if current != indent_str && candidate.chars().count() > INFO_WIDTH {
            lines.push(current);
            current = format!("{indent_str}{token}");
        } else if current == indent_str {
            current.push_str(token);
        } else {
            current.push_str(&gap_str);
            current.push_str(token);
        }
    }
    if current != indent_str {
        lines.push(current);
    }
    lines
}

pub(crate) fn format_source_field(source: Option<&PackageReceiptSource>) -> String {
    match source {
        Some(PackageReceiptSource::Formula { .. }) => "Homebrew".to_string(),
        Some(PackageReceiptSource::Vendor { .. }) => "Subs".to_string(),
        Some(PackageReceiptSource::Npm { .. }) => "npm".to_string(),
        Some(PackageReceiptSource::Pip { .. }) => "PyPI".to_string(),
        None => "Unknown".to_string(),
    }
}

pub(crate) fn format_version_value(info: &PackageInfo) -> String {
    if let Some(installed_version) = info.installed_version.as_deref() {
        installed_version.to_string()
    } else if let Some(latest_version) = info.latest_version.as_deref() {
        latest_version.to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn format_version_status(info: &PackageInfo) -> Option<String> {
    if !info.installed {
        return None;
    }
    match (&info.installed_version, &info.latest_version) {
        (Some(installed_version), Some(latest_version)) if installed_version == latest_version => {
            Some("✔ up to date".to_string())
        }
        (Some(_), Some(latest_version)) => Some(format!("update available ({latest_version})")),
        (_, Some(_)) => None,
        (_, None) => info
            .latest_version_error
            .as_ref()
            .map(|err| format!("latest unknown ({err})")),
    }
}

pub(crate) fn homebrew_formula_page_url(formula: &str) -> String {
    format!("https://formulae.brew.sh/formula/{formula}")
}

pub(crate) fn installed_stub_paths_at(install_root: &Path) -> Result<Vec<String>, String> {
    let mut paths = load_stub_manifest(&install_root.join(STUB_MANIFEST))?
        .stubs
        .into_iter()
        .map(|stub| {
            PathBuf::from(USR_LOCAL_BIN)
                .join(stub)
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
pub(crate) fn installed_package_names(opt_root: &Path) -> Result<Vec<String>, String> {
    Ok(installed_package_refs(opt_root)?
        .into_iter()
        .map(|package| package.package_name)
        .collect())
}

pub(crate) fn installed_package_refs(opt_root: &Path) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(opt_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", opt_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", opt_root.display()))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", opt_root.display()))?;
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        if name == "homebrew" {
            continue;
        }
        if name == "npm" {
            packages.extend(installed_npm_package_refs(&path)?);
            continue;
        }
        if name == "pip" {
            packages.extend(installed_pip_package_refs(&path)?);
            continue;
        }
        packages.push(InstalledPackageRef {
            package_name: name,
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn installed_npm_package_refs(
    npm_root: &Path,
) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(npm_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", npm_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", npm_root.display()))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", npm_root.display()))?;
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        if name.starts_with('@') {
            let scope_entries = fs::read_dir(&path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            for scope_entry in scope_entries {
                let scope_entry = scope_entry
                    .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
                let scoped_path = scope_entry.path();
                let scoped_name = scope_entry
                    .file_name()
                    .into_string()
                    .map_err(|_| format!("non-utf8 directory name under {}", path.display()))?;
                if scoped_name.starts_with('.') || !scoped_path.is_dir() {
                    continue;
                }
                let package = format!("{name}/{scoped_name}");
                packages.push(InstalledPackageRef {
                    package_name: match load_package_receipt(&scoped_path.join(ROOT_RECEIPT)) {
                        Ok(Some(receipt)) => receipt.package_name,
                        Ok(None) | Err(_) => npm_package_display_name(&package),
                    },
                    install_root: scoped_path,
                });
            }
            continue;
        }
        packages.push(InstalledPackageRef {
            package_name: match load_package_receipt(&path.join(ROOT_RECEIPT)) {
                Ok(Some(receipt)) => receipt.package_name,
                Ok(None) | Err(_) => npm_package_display_name(&name),
            },
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn installed_pip_package_refs(
    pip_root: &Path,
) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(pip_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", pip_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", pip_root.display()))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", pip_root.display()))?;
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        packages.push(InstalledPackageRef {
            package_name: match load_package_receipt(&path.join(ROOT_RECEIPT)) {
                Ok(Some(receipt)) => receipt.package_name,
                Ok(None) | Err(_) => pip_package_display_name(&name),
            },
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn load_or_resolve_package_receipt(
    package_name: &str,
    install_root: &Path,
) -> Result<PackageReceipt, String> {
    load_package_receipt(&install_root.join(ROOT_RECEIPT))?
        .ok_or_else(|| format!("package {package_name} is installed but missing package metadata"))
}

pub(crate) fn load_package_receipt(path: &Path) -> Result<Option<PackageReceipt>, String> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let receipt = serde_json::from_slice(&data)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    Ok(Some(receipt))
}

pub(crate) fn write_package_receipt(path: &Path, receipt: &PackageReceipt) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(receipt)
        .map_err(|err| format!("failed to serialize package receipt: {err}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

pub(crate) fn resolve_formula_latest_version(
    config: &Config,
    formula: &str,
) -> Result<String, String> {
    let info = fetch_formula_info(formula)?;
    ensure_formula_has_bottle(formula, &info, &config.bottle_tag)?;
    Ok(formula_version_string(&info))
}

pub(crate) fn resolve_vendor_latest_version(package_name: &str) -> Result<String, String> {
    let package = vendor::get(package_name)
        .ok_or_else(|| format!("vendor package {package_name} is not registered"))?;
    (package.version)().map(|version| version.to_string())
}

pub(crate) fn resolve_npm_package_version(package_name: &str) -> Result<semver::Version, String> {
    let version = vendor::npm_latest_tag(package_name)?;
    vendor::parse_semver(&version, package_name)
}

pub(crate) fn resolve_npm_latest_version(package_name: &str) -> Result<String, String> {
    resolve_npm_package_version(package_name).map(|version| version.to_string())
}

pub(crate) fn resolve_npm_homepage(package_name: &str) -> Result<Option<String>, String> {
    let url = format!(
        "{}/{}",
        std::env::var("PKG_NPM_REGISTRY_ROOT")
            .unwrap_or_else(|_| "https://registry.npmjs.org".to_string()),
        urlencoding::encode(package_name)
    );
    let response: NpmPackageMetadata = fetch_json(&url, || {
        format!("failed to fetch npm metadata for {package_name}")
    })?;
    Ok(response.homepage.and_then(|value| string_or_none(&value)))
}

pub(crate) fn resolve_pip_latest_version(package_name: &str) -> Result<String, String> {
    let normalized = normalize_pip_package_name(package_name);
    let url = format!("{}/{}/json", pypi_root(), urlencoding::encode(&normalized));
    let response: PypiPackageInfoResponse = fetch_json(&url, || {
        format!("failed to fetch PyPI metadata for {package_name}")
    })?;
    if response.info.version.is_empty() {
        return Err(format!(
            "failed to resolve latest PyPI version for {package_name}"
        ));
    }
    Ok(response.info.version)
}

#[cfg(test)]
pub(crate) fn extract_semver_from_text(text: &str) -> Option<semver::Version> {
    for token in text.split_whitespace() {
        let token = token.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && !matches!(ch, '.' | '-' | '+' | '_')
        });
        let token = token.strip_prefix('v').unwrap_or(token);
        if token.is_empty() {
            continue;
        }
        if let Ok(version) = semver::Version::parse(token) {
            return Some(version);
        }
    }
    None
}
