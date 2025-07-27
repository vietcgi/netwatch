#!/bin/bash

# Netwatch Release Management Script
# Automates version bumping, changelog updates, and release creation

set -euo pipefail

# Configuration
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"
CHANGELOG="$REPO_ROOT/CHANGELOG.md"
DOCKERFILE="$REPO_ROOT/Dockerfile"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }
fatal() { error "$1"; exit 1; }

# Get current version from Cargo.toml
get_current_version() {
    grep '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/'
}

# Validate version format (semver)
validate_version() {
    local version="$1"
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$ ]]; then
        fatal "Invalid version format: $version. Must be semantic version (e.g., 1.2.3)"
    fi
}

# Update version in Cargo.toml
update_cargo_version() {
    local new_version="$1"
    info "Updating version in Cargo.toml to $new_version"
    
    # Create backup
    cp "$CARGO_TOML" "$CARGO_TOML.bak"
    
    # Update version
    sed -i.tmp "s/^version = \".*\"/version = \"$new_version\"/" "$CARGO_TOML"
    rm "$CARGO_TOML.tmp"
    
    success "Updated Cargo.toml version to $new_version"
}

# Update version in Dockerfile
update_dockerfile_version() {
    local new_version="$1"
    info "Updating version in Dockerfile to $new_version"
    
    # Create backup
    cp "$DOCKERFILE" "$DOCKERFILE.bak"
    
    # Update version label
    sed -i.tmp "s/LABEL version=\".*\"/LABEL version=\"$new_version\"/" "$DOCKERFILE"
    rm "$DOCKERFILE.tmp"
    
    success "Updated Dockerfile version to $new_version"
}

# Update changelog with new version
update_changelog() {
    local new_version="$1"
    local current_date="$(date +%Y-%m-%d)"
    
    info "Updating CHANGELOG.md for version $new_version"
    
    # Create backup
    cp "$CHANGELOG" "$CHANGELOG.bak"
    
    # Create new changelog entry
    local temp_changelog
    temp_changelog=$(mktemp)
    
    # Add new version header after the main title
    {
        # Keep the main title and description
        head -n 2 "$CHANGELOG"
        echo
        echo "## [$new_version] - $current_date"
        echo
        echo "### Added"
        echo "- "
        echo
        echo "### Changed"
        echo "- "
        echo
        echo "### Fixed"
        echo "- "
        echo
        echo "### Security"
        echo "- "
        echo
        # Add the rest of the existing changelog
        tail -n +3 "$CHANGELOG"
    } > "$temp_changelog"
    
    mv "$temp_changelog" "$CHANGELOG"
    
    warning "Please edit CHANGELOG.md to add the actual changes for version $new_version"
    success "Added changelog template for version $new_version"
}

# Commit changes
commit_changes() {
    local new_version="$1"
    
    info "Committing version changes..."
    
    # Check if there are changes to commit
    if git diff --quiet; then
        warning "No changes to commit"
        return
    fi
    
    # Add files
    git add "$CARGO_TOML" "$CHANGELOG" "$DOCKERFILE"
    
    # Commit
    git commit -m "Release v$new_version

- Update version in Cargo.toml to $new_version
- Update changelog for v$new_version
- Update Dockerfile version label"
    
    success "Committed version changes"
}

# Create and push tag
create_tag() {
    local new_version="$1"
    local tag_name="v$new_version"
    
    info "Creating git tag $tag_name"
    
    # Check if tag already exists
    if git tag -l | grep -q "^$tag_name$"; then
        fatal "Tag $tag_name already exists"
    fi
    
    # Create annotated tag
    git tag -a "$tag_name" -m "Release $tag_name

$(grep -A 20 "## \[$new_version\]" "$CHANGELOG" | tail -n +3 | sed '/^## \[/q' | head -n -1)"
    
    success "Created tag $tag_name"
}

# Push changes and tag
push_release() {
    local new_version="$1"
    local tag_name="v$new_version"
    
    info "Pushing changes and tag to remote..."
    
    # Push changes
    git push origin "$(git branch --show-current)"
    
    # Push tag
    git push origin "$tag_name"
    
    success "Pushed release $tag_name to remote repository"
    info "GitHub Actions will now build and publish the release automatically"
}

# Validate environment
validate_environment() {
    # Check if we're in a git repository
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        fatal "Not in a git repository"
    fi
    
    # Check if working directory is clean
    if ! git diff --quiet || ! git diff --cached --quiet; then
        fatal "Working directory is not clean. Please commit or stash changes first."
    fi
    
    # Check if required files exist
    for file in "$CARGO_TOML" "$CHANGELOG" "$DOCKERFILE"; do
        if [[ ! -f "$file" ]]; then
            fatal "Required file not found: $file"
        fi
    done
    
    # Check if git remote exists
    if ! git remote get-url origin >/dev/null 2>&1; then
        fatal "No git remote 'origin' configured"
    fi
    
    # Check if we can push
    local current_branch
    current_branch=$(git branch --show-current)
    if [[ -z "$current_branch" ]]; then
        fatal "Not on a branch"
    fi
    
    success "Environment validation passed"
}

# Generate next version suggestions
suggest_next_version() {
    local current_version="$1"
    
    # Parse current version
    local major minor patch
    IFS='.' read -r major minor patch <<< "$current_version"
    
    # Remove any pre-release or build metadata from patch
    patch="${patch%%-*}"
    patch="${patch%%+*}"
    
    echo "Current version: $current_version"
    echo
    echo "Suggested next versions:"
    echo "  Patch (bug fixes):     $major.$minor.$((patch + 1))"
    echo "  Minor (new features):  $major.$((minor + 1)).0"
    echo "  Major (breaking):      $((major + 1)).0.0"
}

# Dry run mode
dry_run() {
    local new_version="$1"
    
    info "DRY RUN MODE - No changes will be made"
    echo
    echo "Would perform the following actions:"
    echo "  1. Update Cargo.toml version: $(get_current_version) → $new_version"
    echo "  2. Update Dockerfile version label"
    echo "  3. Add changelog entry for $new_version"
    echo "  4. Commit changes with message: 'Release v$new_version'"
    echo "  5. Create git tag: v$new_version"
    echo "  6. Push changes and tag to remote"
    echo
    warning "Run without --dry-run to actually perform the release"
}

# Print usage
print_usage() {
    cat << EOF
Netwatch Release Management Script

USAGE:
    $0 [OPTIONS] <VERSION>

ARGUMENTS:
    VERSION                 New version number (e.g., 1.2.3)

OPTIONS:
    --dry-run              Show what would be done without making changes
    --no-push              Don't push to remote (useful for testing)
    --help                 Show this help message

EXAMPLES:
    # Suggest next version
    $0

    # Release new patch version
    $0 1.0.1

    # Release new minor version (dry run)
    $0 --dry-run 1.1.0

    # Release but don't push (for testing)
    $0 --no-push 1.0.1

WORKFLOW:
    1. Validates environment and version format
    2. Updates Cargo.toml version
    3. Updates Dockerfile version label
    4. Adds changelog entry template
    5. Commits all changes
    6. Creates annotated git tag
    7. Pushes changes and tag to remote
    8. GitHub Actions automatically builds and publishes

EOF
}

# Main function
main() {
    local new_version=""
    local dry_run_mode=false
    local no_push=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                dry_run_mode=true
                shift
                ;;
            --no-push)
                no_push=true
                shift
                ;;
            --help|-h)
                print_usage
                exit 0
                ;;
            -*)
                error "Unknown option: $1"
                print_usage
                exit 1
                ;;
            *)
                if [[ -n "$new_version" ]]; then
                    error "Multiple version arguments provided"
                    print_usage
                    exit 1
                fi
                new_version="$1"
                shift
                ;;
        esac
    done
    
    # If no version provided, show suggestions
    if [[ -z "$new_version" ]]; then
        local current_version
        current_version=$(get_current_version)
        suggest_next_version "$current_version"
        echo
        echo "Usage: $0 <new_version>"
        exit 0
    fi
    
    # Validate inputs
    validate_version "$new_version"
    validate_environment
    
    local current_version
    current_version=$(get_current_version)
    
    info "Current version: $current_version"
    info "New version: $new_version"
    
    # Check if version is actually newer
    if [[ "$new_version" == "$current_version" ]]; then
        fatal "New version must be different from current version"
    fi
    
    # Dry run mode
    if [[ "$dry_run_mode" == true ]]; then
        dry_run "$new_version"
        exit 0
    fi
    
    # Confirmation
    echo
    warning "This will create a new release with version $new_version"
    read -p "Continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "Release cancelled"
        exit 0
    fi
    
    # Perform release steps
    update_cargo_version "$new_version"
    update_dockerfile_version "$new_version"
    update_changelog "$new_version"
    
    # Open changelog for editing
    if command -v "$EDITOR" >/dev/null 2>&1; then
        warning "Opening changelog for editing..."
        warning "Please add the actual changes for this release"
        read -p "Press Enter to open editor..."
        "$EDITOR" "$CHANGELOG"
    fi
    
    commit_changes "$new_version"
    create_tag "$new_version"
    
    if [[ "$no_push" != true ]]; then
        push_release "$new_version"
        
        echo
        success "🎉 Release v$new_version has been created!"
        echo
        info "Next steps:"
        echo "  1. Monitor the GitHub Actions workflow at:"
        echo "     https://github.com/vietcgi/netwatch/actions"
        echo "  2. Review the generated release at:"
        echo "     https://github.com/vietcgi/netwatch/releases/tag/v$new_version"
        echo "  3. Test the installation script and binaries"
        echo "  4. Announce the release in relevant channels"
    else
        warning "Changes committed and tagged locally but not pushed to remote"
        info "Run 'git push origin $(git branch --show-current) && git push origin v$new_version' to publish"
    fi
}

# Run main function with all arguments
main "$@"