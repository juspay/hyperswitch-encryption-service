#!/usr/bin/env bash
set -Eeuo pipefail

# Change to project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${PROJECT_ROOT}"

# Error logging will go to console only

# Set traps for errors and interruptions
trap 'handle_error "$LINENO" "$BASH_COMMAND" "$?"' ERR
trap 'handle_interrupt' INT TERM

# Variables for installation status
VERSION="unknown"
INSTALLATION_STATUS="initiated"

# Trap and handle any errors that occur during the script execution
handle_error() {
    local lineno=$1
    local last_command=$2
    local exit_code=$3

    # Set global vars
    INSTALLATION_STATUS="error"
    ERROR_MESSAGE="Command '\$ ${last_command}' failed at line ${lineno} with exit code ${exit_code}"

    echo_error "Setup failed: ${ERROR_MESSAGE}"
    cleanup
    exit $exit_code
}

# Handle user interruptions
handle_interrupt() {
    echo ""
    echo_warning "Script interrupted by user"
    INSTALLATION_STATUS="user_interrupt"
    cleanup
    exit 130
}

cleanup() {
    # Clean up any started containers if we've selected a profile
    if [ -n "${PROFILE:-}" ]; then
        echo_info "Cleaning up any started containers..."
        case $PROFILE in
        standalone)
            $DOCKER_COMPOSE -f docker/local/docker-compose.yml down >/dev/null 2>&1 || true
            ;;
        full)
            $DOCKER_COMPOSE -f docker/local/docker-compose.yml --profile monitoring down >/dev/null 2>&1 || true
            ;;
        esac
    fi
}

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Function to print colorful messages
echo_info() {
    printf "${BLUE}[INFO]${NC} %s\n" "$1"
}

echo_success() {
    printf "${GREEN}[SUCCESS]${NC} %s\n" "$1"
}

echo_warning() {
    printf "${YELLOW}[WARNING]${NC} %s\n" "$1"
}

echo_error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
}

show_banner() {
    printf "${BLUE}${BOLD}\n"
    printf "\n"
    printf "   ####  #####  # #####  #####   ##   \n"
    printf "  #    # #    # # #    #   #    #  #  \n"
    printf "  #      #    # # #    #   #   #    # \n"
    printf "  #      #####  # #####    #   ###### \n"
    printf "  #    # #   #  # #        #   #    # \n"
    printf "   ####  #    # # #        #   #    # \n"
    printf "\n"
    printf "  ###### #    #  ####  #####  #   # #####  ##### #  ####  #    # \n"
    printf "  #      ##   # #    # #    #  # #  #    #   #   # #    # ##   # \n"
    printf "  #####  # #  # #      #    #   #   #    #   #   # #    # # #  # \n"
    printf "  #      #  # # #      #####    #   #####    #   # #    # #  # # \n"
    printf "  #      #   ## #    # #   #    #   #        #   # #    # #   ## \n"
    printf "  ###### #    #  ####  #    #   #   #        #   #  ####  #    # \n"
    printf "\n"
    sleep 1
    printf "${NC}\n"
    printf "🔐 ${BLUE}Cripta Encryption Service - Docker Setup${NC} 🔐\n"
}

# Detect Docker Compose version
detect_docker_compose() {
    # Check Docker or Podman
    if command -v docker &>/dev/null; then
        CONTAINER_ENGINE="docker"
        echo_success "Docker is installed."
        echo ""
    elif command -v podman &>/dev/null; then
        CONTAINER_ENGINE="podman"
        echo_success "Podman is installed."
        echo ""
    else
        echo_error "Neither Docker nor Podman is installed. Please install one of them to proceed."
        echo_info "Visit https://docs.docker.com/get-docker/ or https://podman.io/docs/installation for installation instructions."
        echo_info "After installation, re-run this script: scripts/setup.sh"
        echo ""
        exit 1
    fi

    # Check Docker Compose or Podman Compose
    if $CONTAINER_ENGINE compose version &>/dev/null; then
        DOCKER_COMPOSE="${CONTAINER_ENGINE} compose"
        echo_success "Compose is installed for ${CONTAINER_ENGINE}."
        echo ""
    else
        echo_error "Compose is not installed for ${CONTAINER_ENGINE}. Please install ${CONTAINER_ENGINE} compose to proceed."
        echo ""
        if [ "${CONTAINER_ENGINE}" = "docker" ]; then
            echo_info "Visit https://docs.docker.com/compose/install/ for installation instructions."
            echo ""
        elif [ "${CONTAINER_ENGINE}" = "podman" ]; then
            echo_info "Visit https://podman-desktop.io/docs/compose/setting-up-compose for installation instructions."
            echo ""
        fi
        echo_info "After installation, re-run this script: scripts/setup.sh"
        echo ""
        exit 1
    fi
}

check_prerequisites() {
    # Check curl
    if ! command -v curl &>/dev/null; then
        echo_error "curl is not installed. Please install curl to proceed."
        echo ""
        exit 1
    fi
    echo_success "curl is installed."
    echo ""

    # Check ports
    required_ports=(5000 6128 5432)
    unavailable_ports=()

    for port in "${required_ports[@]}"; do
        if command -v nc &>/dev/null; then
            if nc -z localhost "$port" 2>/dev/null; then
                unavailable_ports+=("$port")
            fi
        elif command -v lsof &>/dev/null; then
            if lsof -i :"$port" &>/dev/null; then
                unavailable_ports+=("$port")
            fi
        else
            echo_warning "Neither nc nor lsof available to check ports. Skipping port check."
            echo ""
            break
        fi
    done

    if [ ${#unavailable_ports[@]} -ne 0 ]; then
        echo_warning "The following ports are already in use: ${unavailable_ports[*]}"
        echo_warning "This might cause conflicts with Cripta services."
        echo ""
        echo -n "Do you want to continue anyway? (y/n): "
        read -n 1 -r REPLY
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo_success "All required ports (5000, 6128, 5432) are available."
        echo ""
    fi
}

setup_config() {
    if [ ! -f "config/development.toml" ]; then
        echo_error "Configuration file 'config/development.toml' not found. Please ensure the file exists and is correctly configured."
        exit 1
    fi

    echo_success "Configuration setup complete."
    echo ""
}

select_profile() {
    printf "\n"
    printf "Select a setup option:\n"
    printf "1) ${YELLOW}Standalone Setup${NC}: ${BLUE}[Recommended]${NC} Ideal for API testing and development.\n"
    printf "   Services included: ${BLUE}Cripta Server, PostgreSQL${NC}\n"
    printf "\n"
    printf "2) ${YELLOW}Full Setup${NC}: Ideal for production-like environment with monitoring.\n"
    printf "   Services included: ${BLUE}Everything in Standalone + Prometheus, Grafana${NC}\n"
    printf "\n"
    echo ""
    local profile_selected=false
    while [ "${profile_selected}" = "false" ]; do
        echo -n "Enter your choice (1-2): "
        read -n 1 profile_choice
        echo

        case $profile_choice in
        1)
            PROFILE="standalone"
            profile_selected=true
            ;;
        2)
            PROFILE="full"
            profile_selected=true
            ;;
        *)
            echo_error "Invalid choice. Please enter 1 or 2."
            ;;
        esac
    done

    echo_info "Selected setup: ${PROFILE}"
    echo ""
}

start_services() {
    echo_info "Starting Cripta services..."
    echo ""

    case $PROFILE in
    standalone)
        $DOCKER_COMPOSE -f docker/local/docker-compose.yml up -d pg migration_runner cripta-server
        ;;
    full)
        $DOCKER_COMPOSE -f docker/local/docker-compose.yml --profile monitoring up -d
        ;;
    esac
}

check_services_health() {
    CRIPTA_BASE_URL="http://localhost:5000"
    CRIPTA_HEALTH_URL="${CRIPTA_BASE_URL}/health"
    local is_success=true
    local max_attempts=30
    local attempt=1

    echo_info "Waiting for Cripta services to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl --silent --fail "${CRIPTA_HEALTH_URL}" >/dev/null 2>&1; then
            echo_success "Cripta server is healthy!"
            is_success=true
            break
        else
            echo -n "."
            sleep 2
            attempt=$((attempt + 1))
            is_success=false
        fi
    done
    
    echo ""
    
    if [ "${is_success}" = true ]; then
        VERSION="0.1.0"
        INSTALLATION_STATUS="success"
        print_access_info
    else
        echo_error "Cripta server failed to start properly. Check logs with: ${DOCKER_COMPOSE} -f docker/local/docker-compose.yml logs cripta-server"
        exit 1
    fi
}

print_access_info() {
    printf "${BLUE}"
    printf "╔════════════════════════════════════════════════════════════════╗\n"
    printf "║             Welcome to Cripta Encryption Service!             ║\n"
    printf "╚════════════════════════════════════════════════════════════════╝\n"
    printf "${NC}\n"

    printf "${GREEN}${BOLD}Setup complete! You can now access Cripta services at:${NC}\n"
    printf "  • ${GREEN}${BOLD}API Server${NC}: ${BLUE}${BOLD}http://localhost:5000${NC}\n"
    printf "  • ${GREEN}${BOLD}Metrics${NC}: ${BLUE}${BOLD}http://localhost:6128/metrics${NC}\n"
    printf "  • ${GREEN}${BOLD}Health Check${NC}: ${BLUE}${BOLD}http://localhost:5000/health${NC}\n"

    if [ "$PROFILE" = "full" ]; then
        printf "  • ${GREEN}${BOLD}Monitoring (Grafana)${NC}: ${BLUE}${BOLD}http://localhost:3000${NC}\n"
        printf "  • ${GREEN}${BOLD}Prometheus${NC}: ${BLUE}${BOLD}http://localhost:9090${NC}\n"
    fi
    printf "\n"

    # API usage examples
    printf "${YELLOW}${BOLD}Quick API Test:${NC}\n"
    printf "Create a key: ${BLUE}curl -X POST http://localhost:5000/key -H 'Content-Type: application/json' -d '{\"identifier\":\"test-key\"}'${NC}\n"
    printf "Encrypt data: ${BLUE}curl -X POST http://localhost:5000/data/encrypt -H 'Content-Type: application/json' -d '{\"identifier\":\"test-key\",\"data\":\"SGVsbG8gV29ybGQ=\"}'${NC}\n"
    printf "\n"

    # Provide the stop command based on the selected profile
    echo_info "To stop all services, run the following command:"
    case $PROFILE in
    standalone)
        printf "${BLUE}$DOCKER_COMPOSE -f docker/local/docker-compose.yml down${NC}\n"
        ;;
    full)
        printf "${BLUE}$DOCKER_COMPOSE -f docker/local/docker-compose.yml --profile monitoring down${NC}\n"
        ;;
    esac
    printf "\n"
    printf "For more management options, use: ${BLUE}./scripts/docker/docker-setup.sh${NC}\n"
    printf "\n"
}

# Main execution flow
show_banner
detect_docker_compose
check_prerequisites
setup_config
select_profile
start_services
check_services_health
