//! `iron-build init [<dir>]`: writes a default `project.json` whose templates take their values
//! from the person running it (`"title": "${firstName}'s Game"`), a minimal `src/main.ts` when the
//! folder has no `src/`, and prints what the templates evaluate to right now. See
//! `docs/iron-build.md`.

use crate::icon;
use iron_core::project::{
    FixedUser, LoadOptions, PROJECT_FILE, Platform, Project, SystemUser, UserInfo, file_safe,
};
use std::path::{Path, PathBuf};

/// What `init` writes besides the defaults.
#[derive(Clone, Debug, Default)]
pub struct InitOptions {
    pub name: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub identifier: Option<String>,
    /// `private` (default), `custom` or an SPDX identifier such as `MIT`.
    pub license: Option<String>,
    /// An icon file (`.png`, `.ico`, any image) to copy next to `project.json`.
    pub icon: Option<PathBuf>,
    /// Overwrite an existing `project.json`.
    pub force: bool,
    /// The person the preview is evaluated for (default: the system's user).
    pub user: Option<FixedUser>,
}

/// What `init` did.
#[derive(Debug)]
pub struct InitReport {
    pub project_file: PathBuf,
    /// The project as the templates resolve now.
    pub resolved: Project,
    pub wrote_main: bool,
    pub icon: Option<PathBuf>,
    pub gitignore_updated: bool,
}

const MAIN_TS: &str = "class Hello extends Component {
  override onStart() {
    console.log(\"hello\");
  }
}

register({ Hello });
";

fn io(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// The text of the default `project.json`.
pub fn default_project_json(name: &str, o: &InitOptions, icon: Option<&str>) -> String {
    let q = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into());
    let mut lines = vec![
        format!("  \"name\": {}", q(name)),
        format!(
            "  \"title\": {}",
            q(o.title.as_deref().unwrap_or("${firstName}'s Game"))
        ),
        format!(
            "  \"company\": {}",
            q(o.company.as_deref().unwrap_or("${firstName}'s Company"))
        ),
        format!(
            "  \"identifier\": {}",
            q(o.identifier.as_deref().unwrap_or("com.${username}.${name}"))
        ),
        format!(
            "  \"version\": {}",
            q(o.version.as_deref().unwrap_or("0.1.0"))
        ),
        format!(
            "  \"description\": {}",
            q(o.description.as_deref().unwrap_or("${title}"))
        ),
        format!(
            "  \"license\": {}",
            q(o.license.as_deref().unwrap_or("private"))
        ),
        "  \"copyright\": \"Copyright (c) ${year} ${company}\"".to_owned(),
    ];
    if let Some(i) = icon {
        lines.push(format!("  \"icon\": {}", q(i)));
    }
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

/// Writes the project files into `dir` (created when missing).
pub fn init(
    dir: &Path,
    o: &InitOptions,
    default_user: &dyn UserInfo,
) -> Result<InitReport, String> {
    std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    let project_file = dir.join(PROJECT_FILE);
    if project_file.exists() && !o.force {
        return Err(format!(
            "{} already exists (--force overwrites it)",
            project_file.display()
        ));
    }
    let folder = dir
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "game".into());
    let name = o
        .name
        .clone()
        .unwrap_or_else(|| file_safe(&folder).replace(' ', "-").to_lowercase());

    if let Some(l) = &o.license
        && iron_core::project::canonical_license(l).is_none()
    {
        return Err(format!(
            "--license {l}: unknown license (use `private`, `custom` or one of: {})",
            iron_core::project::SPDX_LICENSES.join(", ")
        ));
    }
    // The icon is copied first, so a bad one stops before anything is written.
    let mut icon_name = None;
    if let Some(src) = &o.icon {
        let data = std::fs::read(src).map_err(|e| io(src, e))?;
        icon::to_ico(&data).map_err(|e| format!("{}: {e}", src.display()))?;
        let file = if icon::is_ico(&data) {
            "icon.ico"
        } else {
            "icon.png"
        };
        let dest = dir.join(file);
        let same = dest.canonicalize().ok() == src.canonicalize().ok();
        if !same {
            std::fs::write(&dest, &data).map_err(|e| io(&dest, e))?;
        }
        icon_name = Some(file);
    }

    std::fs::write(&project_file, default_project_json(&name, o, icon_name))
        .map_err(|e| io(&project_file, e))?;

    let mut wrote_main = false;
    if !dir.join("src").exists() {
        std::fs::create_dir_all(dir.join("src")).map_err(|e| io(&dir.join("src"), e))?;
        let main = dir.join("src/main.ts");
        std::fs::write(&main, MAIN_TS).map_err(|e| io(&main, e))?;
        wrote_main = true;
    }

    let mut gitignore_updated = false;
    let gi = dir.join(".gitignore");
    if let Ok(text) = std::fs::read_to_string(&gi) {
        let has = |p: &str| text.lines().any(|l| l.trim().trim_start_matches('/') == p);
        let missing: Vec<&str> = ["dist/", "*.meta"]
            .into_iter()
            .filter(|p| !has(p))
            .collect();
        if !missing.is_empty() {
            let mut out = text.clone();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            for m in &missing {
                out.push_str(m);
                out.push('\n');
            }
            std::fs::write(&gi, out).map_err(|e| io(&gi, e))?;
            gitignore_updated = true;
        }
    }

    let user: &dyn UserInfo = match &o.user {
        Some(u) => u,
        None => default_user,
    };
    let resolved = Project::load(dir, &LoadOptions::new(Platform::current(), user))
        .map_err(|e| e.to_string())?;
    Ok(InitReport {
        project_file,
        resolved,
        wrote_main,
        icon: icon_name.map(|n| dir.join(n)),
        gitignore_updated,
    })
}

const USAGE: &str =
    "usage: iron-build init [<dir>] [--name n] [--title t] [--company c] [--description d]
       [--icon file.png|ico] [--version v] [--identifier id] [--license private|MIT|...] [--force]";

/// The `init` command; `args` follow the word `init`. Returns the exit code.
pub fn run_cli(args: &[String]) -> i32 {
    let mut o = InitOptions::default();
    let mut dir = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().cloned();
        match a.as_str() {
            "--name" => o.name = value(),
            "--title" => o.title = value(),
            "--company" => o.company = value(),
            "--description" => o.description = value(),
            "--version" => o.version = value(),
            "--identifier" => o.identifier = value(),
            "--license" => o.license = value(),
            "--icon" => o.icon = value().map(PathBuf::from),
            "--force" => o.force = true,
            s if !s.starts_with("--") && dir.is_none() => dir = Some(PathBuf::from(s)),
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    match init(&dir, &o, &SystemUser) {
        Ok(r) => {
            print!("{}", preview(&r));
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

/// The text `init` prints.
pub fn preview(r: &InitReport) -> String {
    let p = &r.resolved;
    let mut out = format!("wrote {}\n", r.project_file.display());
    if r.wrote_main {
        out.push_str("wrote src/main.ts\n");
    }
    if let Some(i) = &r.icon {
        out.push_str(&format!("copied the icon to {}\n", i.display()));
    }
    if r.gitignore_updated {
        out.push_str("added dist/ and *.meta to .gitignore\n");
    }
    out.push_str("as the templates resolve now:\n");
    for (k, v) in [
        ("name", p.name.as_str()),
        ("title", &p.title),
        ("company", &p.company),
        ("identifier", &p.identifier),
        ("version", &p.version),
        ("description", &p.description),
        ("copyright", &p.copyright),
        ("license", &p.license),
        ("exe", &p.exe),
    ] {
        out.push_str(&format!("  {k:<12} {v}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iron_init_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn matthew() -> FixedUser {
        FixedUser {
            username: "mattb".into(),
            full_name: "Matthew Brown".into(),
            date: (2026, 10, 21),
            ..Default::default()
        }
    }

    fn png() -> Vec<u8> {
        let mut out = Vec::new();
        image::RgbaImage::from_pixel(8, 8, image::Rgba([200, 30, 30, 255]))
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    // M9-TS-2700
    #[test]
    fn init_writes_the_default_templates_and_a_hello_and_previews_the_values() {
        let d = temp("default").join("My Fox");
        let r = init(&d, &InitOptions::default(), &matthew()).unwrap();
        let text = std::fs::read_to_string(d.join("project.json")).unwrap();
        assert!(text.contains(r#""title": "${firstName}'s Game""#), "{text}");
        assert!(text.contains(r#""company": "${firstName}'s Company""#));
        assert!(text.contains(r#""license": "private""#));
        assert!(text.contains(r#""identifier": "com.${username}.${name}""#));
        assert!(text.contains("Copyright (c) ${year} ${company}"));
        let p = &r.resolved;
        assert_eq!(p.name, "my-fox");
        assert_eq!(p.title, "Matthew's Game");
        assert_eq!(p.company, "Matthew's Company");
        assert_eq!(p.license, "private");
        assert_eq!(p.identifier, "com.mattb.my-fox");
        assert_eq!(p.copyright, "Copyright (c) 2026 Matthew's Company");
        assert_eq!(p.exe, "Matthews Game");
        assert_eq!(p.version, "0.1.0");
        let main = std::fs::read_to_string(d.join("src/main.ts")).unwrap();
        assert!(main.contains("register({ Hello })") && main.contains("\"hello\""));
        let shown = preview(&r);
        assert!(
            shown.contains("Matthew's Game")
                && shown.contains("Matthew's Company")
                && shown.contains("private"),
            "{shown}"
        );
        // It builds.
        crate::build(
            &d,
            &crate::Options {
                no_tsc: true,
                ..Default::default()
            },
        )
        .unwrap();
        let _ = std::fs::remove_dir_all(d.parent().unwrap());
    }

    // M9-TS-2701
    #[test]
    fn flags_override_only_what_they_name() {
        let d = temp("flags");
        let o = InitOptions {
            company: Some("Quack Games".into()),
            version: Some("2.0.0".into()),
            name: Some("duck".into()),
            description: Some("About ${title}".into()),
            ..Default::default()
        };
        let r = init(&d, &o, &matthew()).unwrap();
        assert_eq!(r.resolved.company, "Quack Games");
        assert_eq!(r.resolved.title, "Matthew's Game");
        assert_eq!(r.resolved.copyright, "Copyright (c) 2026 Quack Games");
        assert_eq!(r.resolved.description, "About Matthew's Game");
        assert_eq!(
            (r.resolved.name.as_str(), r.resolved.version.as_str()),
            ("duck", "2.0.0")
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2702
    #[test]
    fn an_existing_project_json_is_kept_without_force() {
        let d = temp("force");
        init(&d, &InitOptions::default(), &matthew()).unwrap();
        std::fs::write(d.join("project.json"), r#"{ "title": "Mine" }"#).unwrap();
        let e = init(&d, &InitOptions::default(), &matthew()).unwrap_err();
        assert!(e.contains("already exists") && e.contains("--force"), "{e}");
        assert_eq!(
            std::fs::read_to_string(d.join("project.json")).unwrap(),
            r#"{ "title": "Mine" }"#
        );
        let o = InitOptions {
            force: true,
            title: Some("Forced".into()),
            ..Default::default()
        };
        let r = init(&d, &o, &matthew()).unwrap();
        assert_eq!(r.resolved.title, "Forced");
        // An existing src/ is left alone.
        assert!(!r.wrote_main);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2703
    #[test]
    fn the_icon_is_copied_and_gitignore_gets_the_lines_only_when_it_exists() {
        let d = temp("icon");
        std::fs::create_dir_all(&d).unwrap();
        let src = d.join("art.bin");
        std::fs::write(&src, png()).unwrap();
        // No .gitignore: none is made.
        let o = InitOptions {
            icon: Some(src.clone()),
            ..Default::default()
        };
        let r = init(&d, &o, &matthew()).unwrap();
        assert!(d.join("icon.png").exists() && !d.join(".gitignore").exists());
        assert_eq!(r.resolved.icon.as_deref(), Some("icon.png"));
        // With one: the two lines are added once.
        std::fs::write(d.join(".gitignore"), "node_modules").unwrap();
        let o = InitOptions {
            force: true,
            ..Default::default()
        };
        assert!(init(&d, &o, &matthew()).unwrap().gitignore_updated);
        assert!(!init(&d, &o, &matthew()).unwrap().gitignore_updated);
        assert_eq!(
            std::fs::read_to_string(d.join(".gitignore")).unwrap(),
            "node_modules\ndist/\n*.meta\n"
        );
        // A bad icon writes nothing new.
        std::fs::write(&src, b"not an image").unwrap();
        let before = std::fs::read(d.join("project.json")).unwrap();
        let o = InitOptions {
            icon: Some(src),
            force: true,
            title: Some("X".into()),
            ..Default::default()
        };
        assert!(init(&d, &o, &matthew()).is_err());
        assert_eq!(std::fs::read(d.join("project.json")).unwrap(), before);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2705
    #[test]
    fn license_flag_sets_it_and_unknown_values_are_refused() {
        let d = temp("license");
        let o = InitOptions {
            license: Some("mit".into()),
            ..Default::default()
        };
        let r = init(&d, &o, &matthew()).unwrap();
        assert_eq!(r.resolved.license, "MIT");
        assert!(preview(&r).contains("MIT"));
        let o = InitOptions {
            license: Some("Nope-1.0".into()),
            force: true,
            ..Default::default()
        };
        let e = init(&d, &o, &matthew()).unwrap_err();
        assert!(e.contains("--license") && e.contains("Apache-2.0"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2704
    #[test]
    fn a_user_without_a_full_name_gets_the_account_name() {
        let d = temp("noname");
        let u = FixedUser {
            username: "quack".into(),
            ..matthew()
        };
        let u = FixedUser {
            full_name: String::new(),
            ..u
        };
        let r = init(&d, &InitOptions::default(), &u).unwrap();
        assert_eq!(r.resolved.title, "quack's Game");
        assert_eq!(r.resolved.company, "quack's Company");
        let _ = std::fs::remove_dir_all(&d);
    }
}
