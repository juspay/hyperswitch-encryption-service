# Cripta Encryption Service - Docker Setup

This directory contains Docker configurations for running the Cripta encryption service locally.

## 🚀 Quick Start

### Option 1: One-Click Setup (Recommended)
```bash
# From project root
./scripts/docker/setup.sh
```

### Option 2: Manual Setup
```bash
# From project root
./scripts/docker/docker-setup.sh start
```

## 📁 Directory Structure

```
docker/
├── local/
│   ├── docker-compose.yml    # Local development compose file
│   └── Dockerfile.local      # Local development Dockerfile
└── README.md                 # This file
```

## 🛠️ Available Scripts

### Setup Script (`scripts/docker/setup.sh`)
**Purpose**: First-time setup with interactive prompts
- Checks prerequisites (Docker, ports, curl)
- Guides through profile selection
- Performs health checks
- Provides usage examples

**Usage**:
```bash
./scripts/docker/setup.sh
```

### Management Script (`scripts/docker/docker-setup.sh`)
**Purpose**: Day-to-day service management
- Start/stop/restart services
- View logs and status
- Run migrations
- Health checks

**Usage**:
```bash
# Start services
./scripts/docker/docker-setup.sh start [standalone|full]

# Stop services
./scripts/docker/docker-setup.sh stop

# View logs
./scripts/docker/docker-setup.sh logs [service]

# Check health
./scripts/docker/docker-setup.sh health

# See all commands
./scripts/docker/docker-setup.sh help
```

## 🔧 Configuration

### Local Development Config
The Docker setup uses `config/development.toml` which is optimized for local development:
- **TLS disabled** for easier testing
- **Debug logging** enabled
- **Development secrets** (DO NOT use in production)
- **PostgreSQL SSL disabled** for simplicity

### Environment Variables
The following environment variables are set in the Docker containers:
- `CONFIG_DIR=/local/config`
- `CONFIG_FILE=development.toml`
- `RUN_ENV=Dev`
- `RUST_LOG=debug`

## 🏗️ Build Features

### Local Dockerfile (`docker/local/Dockerfile.local`)
- Uses `--features postgres_ssl` for local development
- **Does NOT include mTLS** (easier for local testing)
- Optimized for development workflow

### Production vs Local
| Feature | Local | Production |
|---------|-------|------------|
| TLS/mTLS | ❌ Disabled | ✅ Enabled |
| Build Features | `postgres_ssl` | `release` |
| Secrets | Development | Production |
| Logging | Debug | Info/Warn |

## 🔗 Service Endpoints

### Standalone Setup
- **API Server**: http://localhost:5000
- **Health Check**: http://localhost:5000/health
- **Metrics**: http://localhost:6128/metrics
- **Database**: localhost:5432

### Full Setup (Additional)
- **Grafana**: http://localhost:3000
- **Prometheus**: http://localhost:9090

## 🧪 Testing the API

### Create a Key
```bash
curl -X POST http://localhost:5000/key/create \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic $(echo -n 'secret123:' | base64)" \
  -H "x-tenant-id: public" \
  -d '{"data_identifier": "User", "key_identifier": "test_key_001"}'
```

### Encrypt Data
```bash
curl -X POST http://localhost:5000/data/encrypt \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic $(echo -n 'secret123:' | base64)" \
  -H "x-tenant-id: public" \
  -d '{"data_identifier": "User", "key_identifier": "test_key_001", "data": {"value": "U2VjcmV0RGF0YQo="}}'
```

### Decrypt Data
```bash
curl -X POST http://localhost:5000/data/decrypt \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic $(echo -n 'secret123:' | base64)" \
  -H "x-tenant-id: public" \
  -d '{"data_identifier": "User", "key_identifier": "test_key_001", "data": {"value": "ENCRYPTED_DATA_HERE"}}'
```

## 🔍 Troubleshooting

### Common Issues

#### 1. Port Conflicts
```bash
# Check what's using the ports
lsof -i :5000
lsof -i :6128
lsof -i :5432

# Or use netstat
netstat -tulpn | grep -E ':(5000|6128|5432)'
```

#### 2. Services Won't Start
```bash
# Check logs
./scripts/docker/docker-setup.sh logs cripta-server

# Check service status
./scripts/docker/docker-setup.sh status

# Try rebuilding
./scripts/docker/docker-setup.sh build
```

#### 3. Database Connection Issues
```bash
# Run migrations manually
./scripts/docker/docker-setup.sh migrate

# Check database health
docker compose -f docker/local/docker-compose.yml exec pg pg_isready -U db_user -d encryption_db
```

#### 4. Configuration Issues
- Ensure `config/devlopment.toml` exists
- Check file permissions
- Verify Docker has access to project directory

### Complete Reset
```bash
# Stop and remove everything
./scripts/docker/docker-setup.sh clean

# Start fresh
./scripts/docker/setup.sh
```

## 🔒 Security Notes

### Development vs Production
⚠️ **Important**: This Docker setup is for **local development only**

**Development (this setup)**:
- Plain HTTP (no TLS)
- Hardcoded development secrets
- Debug logging enabled
- Database without SSL

**Production**:
- mTLS enabled
- Secure secret management
- Minimal logging
- SSL/TLS everywhere

### Default Credentials
- **Database**: `db_user` / `db_pass`
- **API Token**: `secret123`
- **Master Key**: Development key (see config/development.toml)

## 🚀 Next Steps

### For Development
1. Use the API endpoints to test encryption/decryption
2. Check out the Postman collection in `postman/`
3. Monitor metrics at http://localhost:6128/metrics
4. Use Grafana (full setup) for visualization

### For Production
1. Create production Docker configuration
2. Use `--features release` for full feature set
3. Configure proper secrets management
4. Enable mTLS and SSL
5. Set up proper monitoring and logging

## 📚 Additional Resources

- [Main README](../README.md) - Project overview
- [Postman Collection](../postman/) - API testing
- [Configuration Guide](../config/) - Configuration options
- [TLS Setup](../scripts/tls/) - Certificate generation
