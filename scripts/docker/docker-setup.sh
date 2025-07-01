#!/usr/bin/env bash
set -euo pipefail

# Change to project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${PROJECT_ROOT}"

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

# Detect Docker Compose
detect_docker_compose() {
    if command -v docker &>/dev/null && docker compose version &>/dev/null; then
        DOCKER_COMPOSE="docker compose -f docker/local/docker-compose.yml"
    elif command -v podman &>/dev/null && podman compose version &>/dev/null; then
        DOCKER_COMPOSE="podman compose -f docker/local/docker-compose.yml"
    else
        echo_error "Neither Docker nor Podman with Compose is available."
        exit 1
    fi
}

show_help() {
    printf "${BLUE}${BOLD}Cripta Docker Management Script${NC}\n"
    printf "\n"
    printf "Usage: $0 [COMMAND] [OPTIONS]\n"
    printf "\n"
    printf "${YELLOW}Commands:${NC}\n"
    printf "  start [profile]    Start services (profile: standalone|full, default: standalone)\n"
    printf "  stop [profile]     Stop services (profile: standalone|full, default: all)\n"
    printf "  restart [profile]  Restart services\n"
    printf "  status             Show status of all services\n"
    printf "  logs [service]     Show logs (service: cripta-server|pg|all, default: all)\n"
    printf "  migrate            Run database migrations manually\n"
    printf "  clean              Stop and remove all containers, networks, and volumes\n"
    printf "  build              Build the Cripta Docker image\n"
    printf "  health             Check health of running services\n"
    printf "  help               Show this help message\n"
    printf "\n"
    printf "${YELLOW}Examples:${NC}\n"
    printf "  $0 start                    # Start standalone setup\n"
    printf "  $0 start full              # Start full setup with monitoring\n"
    printf "  $0 logs cripta-server      # Show Cripta server logs\n"
    printf "  $0 stop                    # Stop all services\n"
    printf "  $0 clean                   # Complete cleanup\n"
    printf "\n"
}

start_services() {
    local profile=${1:-standalone}
    
    echo_info "Starting Cripta services with profile: $profile"
    
    case $profile in
    standalone)
        $DOCKER_COMPOSE up -d pg migration_runner cripta-server
        ;;
    full)
        $DOCKER_COMPOSE --profile monitoring up -d
        ;;
    *)
        echo_error "Invalid profile: $profile. Use 'standalone' or 'full'"
        exit 1
        ;;
    esac
    
    echo_success "Services started successfully!"
    echo_info "Use './scripts/docker/docker-setup.sh status' to check service status"
    echo_info "Use './scripts/docker/docker-setup.sh health' to check service health"
}

stop_services() {
    local profile=${1:-all}
    
    echo_info "Stopping Cripta services..."
    
    case $profile in
    standalone)
        $DOCKER_COMPOSE stop cripta-server migration_runner pg
        ;;
    full)
        $DOCKER_COMPOSE --profile monitoring stop
        ;;
    all|*)
        $DOCKER_COMPOSE --profile monitoring down
        ;;
    esac
    
    echo_success "Services stopped successfully!"
}

restart_services() {
    local profile=${1:-standalone}
    
    echo_info "Restarting Cripta services..."
    stop_services $profile
    sleep 2
    start_services $profile
}

show_status() {
    echo_info "Service Status:"
    $DOCKER_COMPOSE ps
}

show_logs() {
    local service=${1:-all}
    
    case $service in
    all)
        echo_info "Showing logs for all services..."
        $DOCKER_COMPOSE logs -f
        ;;
    cripta-server|pg|migration_runner|prometheus|grafana)
        echo_info "Showing logs for $service..."
        $DOCKER_COMPOSE logs -f $service
        ;;
    *)
        echo_error "Invalid service: $service"
        echo_info "Available services: cripta-server, pg, migration_runner, prometheus, grafana, all"
        exit 1
        ;;
    esac
}

run_migrations() {
    echo_info "Running database migrations..."
    
    # Check if postgres is running
    if ! $DOCKER_COMPOSE ps pg | grep -q "Up"; then
        echo_info "Starting PostgreSQL first..."
        $DOCKER_COMPOSE up -d pg
        echo_info "Waiting for PostgreSQL to be ready..."
        sleep 5
    fi
    
    # Run migrations
    $DOCKER_COMPOSE run --rm migration_runner
    echo_success "Database migrations completed!"
}

clean_all() {
    echo_warning "This will stop and remove all containers, networks, and volumes."
    echo -n "Are you sure? (y/N): "
    read -n 1 -r REPLY
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo_info "Cleaning up all Docker resources..."
        $DOCKER_COMPOSE --profile monitoring down -v --remove-orphans
        
        # Remove any dangling images
        if command -v docker &>/dev/null && docker images -f "dangling=true" -q | grep -q .; then
            echo_info "Removing dangling images..."
            docker rmi $(docker images -f "dangling=true" -q) 2>/dev/null || true
        fi
        
        echo_success "Cleanup completed!"
    else
        echo_info "Cleanup cancelled."
    fi
}

build_image() {
    echo_info "Building Cripta Docker image..."
    $DOCKER_COMPOSE build cripta-server
    echo_success "Image built successfully!"
}

check_health() {
    echo_info "Checking service health..."
    
    # Check if services are running
    if ! $DOCKER_COMPOSE ps cripta-server | grep -q "Up"; then
        echo_error "Cripta server is not running"
        return 1
    fi
    
    # Check health endpoints
    local health_url="http://localhost:5000/health"
    local metrics_url="http://localhost:6128/metrics"
    
    echo_info "Checking API health..."
    if curl -s -f "$health_url" >/dev/null; then
        echo_success "✓ API server is healthy"
    else
        echo_error "✗ API server health check failed"
    fi
    
    echo_info "Checking metrics endpoint..."
    if curl -s -f "$metrics_url" >/dev/null; then
        echo_success "✓ Metrics endpoint is healthy"
    else
        echo_error "✗ Metrics endpoint health check failed"
    fi
    
    # Check database connectivity
    echo_info "Checking database connectivity..."
    if $DOCKER_COMPOSE exec -T pg pg_isready -U db_user -d encryption_db >/dev/null 2>&1; then
        echo_success "✓ Database is healthy"
    else
        echo_error "✗ Database health check failed"
    fi
    
    echo_info "Health check completed!"
}

# Main script logic
detect_docker_compose

case ${1:-help} in
start)
    start_services ${2:-standalone}
    ;;
stop)
    stop_services ${2:-all}
    ;;
restart)
    restart_services ${2:-standalone}
    ;;
status)
    show_status
    ;;
logs)
    show_logs ${2:-all}
    ;;
migrate)
    run_migrations
    ;;
clean)
    clean_all
    ;;
build)
    build_image
    ;;
health)
    check_health
    ;;
help|--help|-h)
    show_help
    ;;
*)
    echo_error "Unknown command: $1"
    echo_info "Use '$0 help' to see available commands"
    exit 1
    ;;
esac
