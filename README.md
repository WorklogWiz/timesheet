# Introduction

This is either the Rust Baseline or a repository based on it. The purpose of the baseline is to quickly get setup with a repository that supports

- **Versioning**

  Using [Semantic Release](https://semantic-release.gitbook.io/semantic-release) the repository will get correctly versioned crates and binaries. The CI pipeline will also have access to this version so it can tag container images accordingly.

- **Static analysis**

  The builds will run clippy so that the repository can stay clean from the beginning. Sorting out these things at a later point is extremely painful and will usually result in a lot of suppressions, which defeats the purpose of static analysis.

- **Code coverage**

  Using [SonarQube](https://docs.sonarsource.com/sonarqube/latest/) and OpenCover the tests are instrumented and results are posted to SonarQube

- **Development container**

  The baseline will be pre-configured with how to setup a developer container, which gives developers a consistent experience for how to build and test, thus removing any machine specific issues.

- **CI/CD pipeline**

  Using [GitLab](https://docs.gitlab.com), the baseline is setup with a pipeline that will run the steps needed for most projects. If you need customizations, it is wise to add those in new files inside the `.gitlab` folder and include them in the `.gitlab-ci.yml`. The rationale for this is that it makes merging updates from the baseline, easier.

## Add new code

To add packages, simply run `cargo new <name of package>`. Then add this as a member of the root `Cargo.toml` file.

## Build and test the entire workspace

To build the workspace, run `cargo build` and to then run the tests, `cargo test`
