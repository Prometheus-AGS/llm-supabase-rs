# Docker Deployment Guide

## Quick Start

### 1. Prerequisites
- Docker and Docker Compose installed
- GCP service account credentials JSON file
- Supabase project credentials

### 2. Configuration Setup

1. **Copy environment template:**
   ```bash
   cp .env.docker .env
   ```

2. **Edit `.env` file with your credentials:**
   ```bash
   # Required: GCP Configuration
   GCP_PROJECT_ID=your-gcp-project-id
   GCP_LOCATION=us-east5
   
   # Required: Supabase Configuration  
   SUPABASE_URL=https://your-project.supabase.co
   SUPABASE_ANON_KEY=your-anon-key
   SUPABASE_SERVICE_ROLE_KEY=your-service-role-key
   SUPABASE_JWT_SECRET=your-jwt-secret
   ```

3. **Place your GCP credentials:**
   ```bash
   # Place your service account JSON file as:
   ./gcp-credentials.json
   ```

### 3. Build and Run

**Option A: Using Docker Compose (Recommended)**
```bash
# Build and start the service
docker-compose up --build

# Run in background
docker-compose up -d --build

# View logs
docker-compose logs -f llm-supabase-rs

# Stop the service
docker-compose down
```

**Option B: Using Docker directly**
```bash
# Build the image
docker build -t llm-supabase-rs .

# Run the container
docker run -d \
  --name llm-supabase-rs \
  -p 8080:8080 \
  --env-file .env \
  -v $(pwd)/gcp-credentials.json:/app/credentials/gcp-credentials.json:ro \
  llm-supabase-rs
```

### 4. Verify Deployment

**Health Check:**
```bash
curl http://localhost:8080/health
```

**Test Chat Completion:**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-sonnet-4-5@20250929",
    "messages": [
      {
        "role": "user",
        "content": "Hello, how are you?"
      }
    ]
  }'
```

**Test Streaming:**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-sonnet-4-5@20250929",
    "stream": true,
    "messages": [
      {
        "role": "user",
        "content": "Tell me something interesting"
      }
    ]
  }'
```

## Production Deployment

### Environment Variables

| Variable | Required | Description | Default |
|----------|----------|-------------|---------|
| `GCP_PROJECT_ID` | ✅ | Google Cloud Project ID | - |
| `GCP_LOCATION` | ✅ | GCP Region | `us-east5` |
| `SUPABASE_URL` | ✅ | Supabase project URL | - |
| `SUPABASE_ANON_KEY` | ✅ | Supabase anonymous key | - |
| `SUPABASE_SERVICE_ROLE_KEY` | ✅ | Supabase service role key | - |
| `SUPABASE_JWT_SECRET` | ✅ | JWT secret for validation | - |
| `HOST` | ❌ | Server bind address | `0.0.0.0` |
| `PORT` | ❌ | Server port | `8080` |
| `LOG_LEVEL` | ❌ | Logging level | `info` |
| `DEFAULT_MODEL` | ❌ | Default AI model | `claude-sonnet-4-5@20250929` |

### Security Considerations

1. **Credentials Management:**
   - Never commit credentials to version control
   - Use Docker secrets in production
   - Rotate credentials regularly

2. **Network Security:**
   - Use HTTPS in production (nginx proxy included)
   - Configure proper firewall rules
   - Limit access to necessary ports only

3. **Container Security:**
   - Runs as non-root user (appuser)
   - Minimal Debian base image
   - Regular security updates

### Monitoring

**Health Checks:**
- Built-in health endpoint: `/health`
- Docker health checks configured
- 30-second intervals with 3 retries

**Logs:**
```bash
# View real-time logs
docker-compose logs -f llm-supabase-rs

# View specific number of lines
docker-compose logs --tail=100 llm-supabase-rs
```

**Metrics:**
- Application metrics available (if enabled)
- Container resource usage via Docker stats
- Custom observability can be added

### Scaling

**Horizontal Scaling:**
```yaml
# In docker-compose.yaml
services:
  llm-supabase-rs:
    deploy:
      replicas: 3
```

**Resource Limits:**
```yaml
# Already configured in docker-compose.yaml
deploy:
  resources:
    limits:
      memory: 1G
      cpus: '0.5'
```

### Troubleshooting

**Common Issues:**

1. **Build Failures:**
   ```bash
   # Clean Docker cache
   docker system prune -a
   
   # Rebuild without cache
   docker-compose build --no-cache
   ```

2. **Authentication Errors:**
   - Verify GCP credentials file exists and is valid
   - Check GCP project ID and region
   - Ensure proper IAM permissions

3. **Connection Issues:**
   - Verify Supabase credentials
   - Check network connectivity
   - Review firewall settings

4. **Performance Issues:**
   - Increase memory limits
   - Monitor CPU usage
   - Check network latency to GCP/Supabase

**Debug Mode:**
```bash
# Run with debug logging
docker-compose up --build -e LOG_LEVEL=debug
```

## SSL/HTTPS Setup (Optional)

The docker-compose.yaml includes an optional nginx service for SSL termination:

1. **Create nginx.conf:**
   ```nginx
   events {
       worker_connections 1024;
   }
   
   http {
       upstream app {
           server llm-supabase-rs:8080;
       }
       
       server {
           listen 80;
           return 301 https://$server_name$request_uri;
       }
       
       server {
           listen 443 ssl;
           server_name your-domain.com;
           
           ssl_certificate /etc/nginx/ssl/cert.pem;
           ssl_certificate_key /etc/nginx/ssl/key.pem;
           
           location / {
               proxy_pass http://app;
               proxy_set_header Host $host;
               proxy_set_header X-Real-IP $remote_addr;
           }
       }
   }
   ```

2. **Enable nginx service:**
   ```bash
   docker-compose --profile with-nginx up -d
   ```

## Maintenance

**Updates:**
```bash
# Pull latest changes
git pull

# Rebuild and restart
docker-compose up --build -d

# Clean old images
docker image prune -f
```

**Backups:**
- Configuration files (`.env`, `docker-compose.yaml`)
- GCP credentials
- Application logs (if persistent storage configured)

This deployment setup provides a production-ready environment with proper security, monitoring, and scalability considerations.