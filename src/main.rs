use clap::{Arg, Command};
use std::process;

mod client;
mod commands;
mod config;
mod constants;
mod filtering;
mod formatting;
mod interactive;
mod models;

use commands::*;

#[tokio::main]
async fn main() {
    let app = Command::new("linear")
        .about("Linear CLI - Interact with Linear's API from the command line")
        .version("1.0.0")
        .subcommand(
            Command::new("auth")
                .about("Authenticate with Linear")
                .arg(
                    Arg::new("api-key")
                        .long("api-key")
                        .value_name("KEY")
                        .help("Set your Linear API key")
                        .conflicts_with("show"),
                )
                .arg(
                    Arg::new("show")
                        .long("show")
                        .help("Show the current API key (masked)")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("issues")
                .about("List and filter issues")
                .arg(
                    Arg::new("mine")
                        .long("mine")
                        .help("Show only issues assigned to you")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("todo")
                        .long("todo")
                        .help("Show only todo/backlog issues")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("backlog")
                        .long("backlog")
                        .help("Show only backlog issues (alias for --todo)")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("triage")
                        .long("triage")
                        .help("Show only triage issues")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("progress")
                        .long("progress")
                        .help("Show only in-progress issues")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("started")
                        .long("started")
                        .help("Show only started issues (alias for --progress)")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("done")
                        .long("done")
                        .help("Show only completed issues")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("completed")
                        .long("completed")
                        .help("Show only completed issues (alias for --done)")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("assignee")
                        .long("assignee")
                        .value_name("EMAIL")
                        .help("Filter by assignee email"),
                )
                .arg(
                    Arg::new("team")
                        .long("team")
                        .value_name("TEAM_KEY")
                        .help("Filter by team key (e.g., ENG)"),
                )
                .arg(
                    Arg::new("search")
                        .short('s')
                        .long("search")
                        .value_name("QUERY")
                        .help("Search issues by title"),
                )
                .arg(
                    Arg::new("filter")
                        .short('f')
                        .long("filter")
                        .value_name("QUERY")
                        .help(
                            r#"Advanced filter query. Examples:
  'assignee:john@example.com AND priority:>2'
  'title:~bug AND created:>1week'
  'has-label:urgent AND state:started'
  'no-assignee AND updated:<2days'

Available operators:
  : (equals), :> (greater than), :< (less than)
  :~ (contains), :!= (not equals), :in (in list)

Special filters:
  has-assignee, no-assignee, has-label:name, no-label

Date values support relative dates: 1hour, 2days, 1week, 1month"#,
                        ),
                )
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .value_name("NUMBER")
                        .help("Limit the number of results (default: 50)")
                        .default_value("50"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .value_name("FORMAT")
                        .help("Output format: simple, table, json")
                        .value_parser(["simple", "table", "json"])
                        .default_value("simple"),
                )
                .arg(
                    Arg::new("group-by")
                        .long("group-by")
                        .value_name("FIELD")
                        .help("Group issues by: status (default), project")
                        .value_parser(["status", "project"])
                        .default_value("status"),
                ),
        )
        .subcommand(
            Command::new("create")
                .about("Create Linear resources")
                .subcommand_required(true)
                .subcommand(
                    Command::new("issue")
                        .about("Create a new issue")
                        .arg(
                            Arg::new("title")
                                .value_name("TITLE")
                                .help("Issue title")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("description")
                                .value_name("DESCRIPTION")
                                .help("Issue description")
                                .index(2),
                        )
                        .arg(
                            Arg::new("team")
                                .short('t')
                                .long("team")
                                .value_name("TEAM_KEY")
                                .help("Team key (e.g., ENG)"),
                        )
                        .arg(
                            Arg::new("priority")
                                .short('p')
                                .long("priority")
                                .value_name("LEVEL")
                                .help("Priority level: none/0, low/1, medium/2, high/3, urgent/4"),
                        )
                        .arg(
                            Arg::new("assignee")
                                .short('a')
                                .long("assignee")
                                .value_name("USER_ID")
                                .help("Assignee user ID"),
                        )
                        .arg(
                            Arg::new("labels")
                                .short('l')
                                .long("labels")
                                .value_name("LABEL_IDS")
                                .help("Label IDs (can be specified multiple times)")
                                .action(clap::ArgAction::Append),
                        ),
                )
                .subcommand(
                    Command::new("project")
                        .about("Create a new project")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Project name")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("description")
                                .value_name("DESCRIPTION")
                                .help("Project description")
                                .index(2),
                        )
                        .arg(
                            Arg::new("teams")
                                .short('t')
                                .long("teams")
                                .value_name("TEAM_IDS")
                                .help("Team IDs (can be specified multiple times)")
                                .action(clap::ArgAction::Append),
                        ),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("Update Linear resources")
                .subcommand_required(true)
                .subcommand(
                    Command::new("issue")
                        .about("Update an existing issue")
                        .arg(
                            Arg::new("id")
                                .value_name("ISSUE_ID")
                                .help("Issue ID or identifier")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("title")
                                .short('t')
                                .long("title")
                                .value_name("TITLE")
                                .help("New title"),
                        )
                        .arg(
                            Arg::new("description")
                                .short('d')
                                .long("description")
                                .value_name("DESCRIPTION")
                                .help("New description"),
                        )
                        .arg(
                            Arg::new("state")
                                .short('s')
                                .long("state")
                                .value_name("STATE_ID")
                                .help("New state ID"),
                        )
                        .arg(
                            Arg::new("priority")
                                .short('p')
                                .long("priority")
                                .value_name("LEVEL")
                                .help("Priority level: none/0, low/1, medium/2, high/3, urgent/4"),
                        )
                        .arg(
                            Arg::new("assignee")
                                .short('a')
                                .long("assignee")
                                .value_name("USER_ID")
                                .help("New assignee user ID"),
                        )
                        .arg(
                            Arg::new("labels")
                                .short('l')
                                .long("labels")
                                .value_name("LABEL_IDS")
                                .help("New label IDs (can be specified multiple times)")
                                .action(clap::ArgAction::Append),
                        ),
                )
                .subcommand(
                    Command::new("project")
                        .about("Update an existing project")
                        .arg(
                            Arg::new("id")
                                .value_name("PROJECT_ID")
                                .help("Project ID")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .value_name("NAME")
                                .help("New name"),
                        )
                        .arg(
                            Arg::new("description")
                                .short('d')
                                .long("description")
                                .value_name("DESCRIPTION")
                                .help("New description"),
                        )
                        .arg(
                            Arg::new("state")
                                .short('s')
                                .long("state")
                                .value_name("STATE")
                                .help("New state: planned, started, paused, completed, canceled"),
                        ),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete (archive) Linear resources")
                .subcommand_required(true)
                .subcommand(
                    Command::new("issue").about("Archive an issue").arg(
                        Arg::new("id")
                            .value_name("ISSUE_ID")
                            .help("Issue ID to archive")
                            .required(true)
                            .index(1),
                    ),
                )
                .subcommand(
                    Command::new("project").about("Archive a project").arg(
                        Arg::new("id")
                            .value_name("PROJECT_ID")
                            .help("Project ID to archive")
                            .required(true)
                            .index(1),
                    ),
                ),
        )
        .subcommand(Command::new("teams").about("List teams"))
        .subcommand(Command::new("projects").about("List projects"))
        .subcommand(Command::new("whoami").about("Show current user information"))
        .subcommand(
            Command::new("issue")
                .about("View a single issue with full details")
                .arg(
                    Arg::new("identifier")
                        .value_name("ISSUE_ID")
                        .help("Issue identifier (e.g., ENG-123)")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("bulk")
                .about("Perform bulk operations on issues")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("update")
                        .about("Update multiple issues at once")
                        .arg(
                            Arg::new("ids")
                                .value_name("ISSUE_IDS")
                                .help("Issue IDs (comma-separated or multiple values)")
                                .required(true)
                                .action(clap::ArgAction::Append)
                                .index(1),
                        )
                        .arg(
                            Arg::new("state")
                                .long("state")
                                .value_name("STATE_ID")
                                .help("New state for all issues"),
                        )
                        .arg(
                            Arg::new("assignee")
                                .long("assignee")
                                .value_name("USER_ID")
                                .help("New assignee for all issues"),
                        )
                        .arg(
                            Arg::new("priority")
                                .long("priority")
                                .value_name("PRIORITY")
                                .help("New priority (0-4) for all issues"),
                        )
                        .arg(
                            Arg::new("labels")
                                .long("labels")
                                .value_name("LABEL_IDS")
                                .help("Add labels to all issues (comma-separated)"),
                        )
                        .arg(
                            Arg::new("remove-labels")
                                .long("remove-labels")
                                .value_name("LABEL_IDS")
                                .help("Remove labels from all issues (comma-separated)"),
                        ),
                )
                .subcommand(
                    Command::new("move")
                        .about("Move multiple issues to a different team/project")
                        .arg(
                            Arg::new("ids")
                                .value_name("ISSUE_IDS")
                                .help("Issue IDs (comma-separated or multiple values)")
                                .required(true)
                                .action(clap::ArgAction::Append)
                                .index(1),
                        )
                        .arg(
                            Arg::new("team")
                                .long("team")
                                .value_name("TEAM_ID")
                                .help("Move to this team"),
                        )
                        .arg(
                            Arg::new("project")
                                .long("project")
                                .value_name("PROJECT_ID")
                                .help("Move to this project"),
                        ),
                )
                .subcommand(
                    Command::new("archive")
                        .about("Archive multiple issues at once")
                        .arg(
                            Arg::new("ids")
                                .value_name("ISSUE_IDS")
                                .help("Issue IDs to archive (comma-separated or multiple values)")
                                .required(true)
                                .action(clap::ArgAction::Append)
                                .index(1),
                        ),
                ),
        )
        .subcommand(
            Command::new("search")
                .about("Manage saved searches")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("save")
                        .about("Save a search query")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Name for the saved search")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("query")
                                .value_name("QUERY")
                                .help("Filter query to save")
                                .required(true)
                                .index(2),
                        ),
                )
                .subcommand(Command::new("list").about("List all saved searches"))
                .subcommand(
                    Command::new("delete").about("Delete a saved search").arg(
                        Arg::new("name")
                            .value_name("NAME")
                            .help("Name of the saved search to delete")
                            .required(true)
                            .index(1),
                    ),
                )
                .subcommand(
                    Command::new("run")
                        .about("Run a saved search")
                        .arg(
                            Arg::new("name")
                                .value_name("NAME")
                                .help("Name of the saved search to run")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .value_name("FORMAT")
                                .help("Output format: simple, table, json")
                                .value_parser(["simple", "table", "json"])
                                .default_value("simple"),
                        )
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .value_name("NUMBER")
                                .help("Limit the number of results (default: 50)")
                                .default_value("50"),
                        ),
                ),
        )
        .subcommand(
            Command::new("comment")
                .about("Manage issue comments")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("list").about("List comments on an issue").arg(
                        Arg::new("issue")
                            .value_name("ISSUE_ID")
                            .help("Issue identifier (e.g., ENG-123)")
                            .required(true)
                            .index(1),
                    ),
                )
                .subcommand(
                    Command::new("add")
                        .about("Add a comment to an issue")
                        .arg(
                            Arg::new("issue")
                                .value_name("ISSUE_ID")
                                .help("Issue identifier (e.g., ENG-123)")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("body")
                                .value_name("COMMENT")
                                .help("Comment text (supports markdown)")
                                .required(true)
                                .index(2),
                        ),
                )
                .subcommand(
                    Command::new("update")
                        .about("Update an existing comment")
                        .arg(
                            Arg::new("id")
                                .value_name("COMMENT_ID")
                                .help("Comment ID to update")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("body")
                                .value_name("COMMENT")
                                .help("New comment text (supports markdown)")
                                .required(true)
                                .index(2),
                        ),
                )
                .subcommand(
                    Command::new("delete").about("Delete a comment").arg(
                        Arg::new("id")
                            .value_name("COMMENT_ID")
                            .help("Comment ID to delete")
                            .required(true)
                            .index(1),
                    ),
                ),
        )
        .subcommand(
            Command::new("git")
                .about("Git integration with Linear")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("commit")
                        .about("Create a commit with Linear issue reference")
                        .arg(
                            Arg::new("message")
                                .value_name("MESSAGE")
                                .help("Commit message")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("issue")
                                .short('i')
                                .long("issue")
                                .value_name("ISSUE_ID")
                                .help("Linear issue ID (e.g., ENG-123)"),
                        )
                        .arg(
                            Arg::new("push")
                                .short('p')
                                .long("push")
                                .help("Push after committing")
                                .action(clap::ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("update-status")
                                .short('u')
                                .long("update-status")
                                .help("Update Linear issue status")
                                .action(clap::ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("status")
                                .short('s')
                                .long("status")
                                .value_name("STATE")
                                .help("New status for the issue")
                                .requires("update-status"),
                        ),
                )
                .subcommand(
                    Command::new("branch")
                        .about("Create a branch from a Linear issue")
                        .arg(
                            Arg::new("issue")
                                .value_name("ISSUE_ID")
                                .help("Linear issue ID (e.g., ENG-123)")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("prefix")
                                .short('p')
                                .long("prefix")
                                .value_name("PREFIX")
                                .help("Branch prefix (default: feature)")
                                .default_value("feature"),
                        ),
                )
                .subcommand(
                    Command::new("pr")
                        .about("Create a pull request linked to Linear issue")
                        .arg(
                            Arg::new("title")
                                .short('t')
                                .long("title")
                                .value_name("TITLE")
                                .help("PR title (defaults to issue title)"),
                        )
                        .arg(
                            Arg::new("body")
                                .short('b')
                                .long("body")
                                .value_name("BODY")
                                .help("PR body (defaults to issue description)"),
                        )
                        .arg(
                            Arg::new("draft")
                                .short('d')
                                .long("draft")
                                .help("Create as draft PR")
                                .action(clap::ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("web")
                                .short('w')
                                .long("web")
                                .help("Open PR in web browser")
                                .action(clap::ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("hook").about("Git hook integration (for commit-msg hook)"),
                )
                .subcommand(Command::new("install-hook").about("Install the commit-msg git hook")),
        )
        .subcommand(
            Command::new("interactive")
                .about("Enter interactive mode for managing Linear issues")
                .visible_alias("i"),
        );

    let matches = app.get_matches();

    let result = match matches.subcommand() {
        None => interactive::handlers::run_interactive_mode().await,
        Some(("auth", sub_matches)) => handle_auth(sub_matches).await,
        Some(("issues", sub_matches)) => handle_issues(sub_matches).await,
        Some(("create", sub_matches)) => match sub_matches.subcommand() {
            Some(("issue", issue_matches)) => handle_create_issue(issue_matches).await,
            Some(("project", project_matches)) => handle_create_project(project_matches).await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("update", sub_matches)) => match sub_matches.subcommand() {
            Some(("issue", issue_matches)) => handle_update_issue(issue_matches).await,
            Some(("project", project_matches)) => handle_update_project(project_matches).await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("delete", sub_matches)) => match sub_matches.subcommand() {
            Some(("issue", issue_matches)) => handle_delete(issue_matches, "Issue").await,
            Some(("project", project_matches)) => handle_delete(project_matches, "Project").await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("teams", sub_matches)) => handle_teams(sub_matches).await,
        Some(("projects", sub_matches)) => handle_projects(sub_matches).await,
        Some(("whoami", sub_matches)) => handle_whoami(sub_matches).await,
        Some(("issue", sub_matches)) => handle_issue(sub_matches).await,
        Some(("search", sub_matches)) => match sub_matches.subcommand() {
            Some(("save", search_matches)) => handle_save_search(search_matches).await,
            Some(("list", _)) => handle_list_searches().await,
            Some(("delete", search_matches)) => handle_delete_search(search_matches).await,
            Some(("run", search_matches)) => handle_run_search(search_matches).await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("bulk", sub_matches)) => match sub_matches.subcommand() {
            Some(("update", bulk_matches)) => handle_bulk_update(bulk_matches).await,
            Some(("move", bulk_matches)) => handle_bulk_move(bulk_matches).await,
            Some(("archive", bulk_matches)) => handle_bulk_archive(bulk_matches).await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("comment", sub_matches)) => match sub_matches.subcommand() {
            Some(("list", comment_matches)) => handle_list_comments(comment_matches).await,
            Some(("add", comment_matches)) => handle_add_comment(comment_matches).await,
            Some(("update", comment_matches)) => handle_update_comment(comment_matches).await,
            Some(("delete", comment_matches)) => handle_delete_comment(comment_matches).await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("git", sub_matches)) => match sub_matches.subcommand() {
            Some(("commit", git_matches)) => handle_git_commit(git_matches).await,
            Some(("branch", git_matches)) => handle_git_branch(git_matches).await,
            Some(("pr", git_matches)) => handle_git_pr(git_matches).await,
            Some(("hook", git_matches)) => handle_git_hook(git_matches).await,
            Some(("install-hook", git_matches)) => handle_install_hook(git_matches).await,
            _ => unreachable!("Subcommand required"),
        },
        Some(("interactive", _)) => interactive::handlers::run_interactive_mode().await,
        _ => unreachable!("Subcommand required"),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
