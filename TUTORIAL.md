# Apicurio CLI Tutorial

This tutorial walks through common usage patterns and provides practical examples for the Apicurio CLI.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Workflow](#basic-workflow)
3. [Configuration Examples](#configuration-examples)
4. [Advanced Usage](#advanced-usage)
5. [Publishing Workflows](#publishing-workflows)
6. [Team Collaboration](#team-collaboration)
7. [CI/CD Integration](#cicd-integration)

## Getting Started

### Installation

Install the Apicurio CLI using one of these methods:

```bash
# From source (requires Rust)
git clone https://github.com/YassineElbouchaibi/apicurio-cli.git
cd apicurio-cli
cargo install --path .

# From cargo (when published)
cargo install apicurio-cli

# Download binary release
curl -L https://github.com/YassineElbouchaibi/apicurio-cli/releases/latest/download/apicurio-linux-x64.tar.gz | tar xz
```

### Initial Setup

Set up your first project:

```bash
# Create a new project directory
mkdir my-api-project
cd my-api-project

# Initialize Apicurio configuration
apicurio init

# This creates:
# - apicurioconfig.yaml (project configuration)
# - apicuriolock.yaml (empty lock file)
```

## Basic Workflow

### 1. Configure a Registry

Add a registry to your global configuration:

```bash
# Add a registry
apicurio registry add company-prod https://registry.company.com

# List configured registries
apicurio registry list

# The registry is saved to ~/.config/apicurio/registries.yaml
```

### 2. Add Dependencies

Add schema dependencies to your project:

```bash
# Interactive mode - prompts for missing information
apicurio add

# Specify full identifier
apicurio add company-prod/com.example.services/user-service@^1.0.0

# Partial identifier - will prompt for missing parts
apicurio add user-service@^1.0.0
```

### 3. Fetch Dependencies

Pull all configured dependencies:

```bash
# Fetch dependencies according to lock file
# If no lock exists, resolves versions and creates lock
apicurio pull

# Check what would be updated
apicurio status

# Update to latest matching versions
apicurio update
```

### 4. Verify Integrity

Ensure downloaded files haven't been corrupted:

```bash
# Verify checksums against lock file
apicurio verify

# Validate configuration and connectivity
apicurio doctor
```

## Configuration Examples

### Project Configuration (`apicurioconfig.yaml`)

Here are examples for different project types:

#### Microservice with Protobuf APIs

```yaml
externalRegistriesFile: ${APICURIO_REGISTRIES_PATH:-}

registries:
  - name: local-dev
    url: http://localhost:8080
    auth:
      type: none

dependencies:
  # Core service definitions
  - name: user-service-api
    groupId: com.company.services
    artifactId: user-service
    version: ^2.1.0
    registry: company-prod
    outputPath: protos/user-service.proto

  # Common types
  - name: common-types
    groupId: com.company.common
    artifactId: types
    version: ^1.5.0
    registry: company-prod
    outputPath: protos/common/types.proto

  # Event schemas
  - name: user-events
    groupId: com.company.events
    artifactId: user-events
    version: ~3.2.0
    registry: company-prod
    outputPath: schemas/user-events.avsc

publishes:
  - name: com.company.services/payment-service
    inputPath: protos/payment-service.proto
    version: 1.0.0
    registry: company-prod
    type: protobuf
    description: "Payment service API definitions"
    labels:
      team: payments
      service: payment-service
    references:
      - name: com.company.common/types
        version: 1.5.2
```

#### Data Pipeline with Avro Schemas

```yaml
dependencies:
  # Input schemas
  - name: raw-events
    groupId: com.company.data.raw
    artifactId: events
    version: ^1.0.0
    registry: data-registry
    outputPath: schemas/raw/events.avsc

  # Processed schemas  
  - name: processed-events
    groupId: com.company.data.processed
    artifactId: events
    version: ^2.0.0
    registry: data-registry
    outputPath: schemas/processed/events.avsc

publishes:
  # Output schema
  - name: com.company.data.enriched/events
    inputPath: schemas/enriched-events.avsc
    version: 1.2.0
    registry: data-registry
    type: avro
    description: "Enriched event schema for analytics"
    labels:
      pipeline: event-enrichment
      format: avro
```

#### API Gateway with OpenAPI Specs

```yaml
dependencies:
  # Service API specifications
  - name: user-api
    groupId: com.company.apis
    artifactId: user-service
    version: ^3.0.0
    registry: api-registry
    outputPath: specs/user-service.yaml

  - name: payment-api
    groupId: com.company.apis
    artifactId: payment-service
    version: ^2.1.0
    registry: api-registry
    outputPath: specs/payment-service.yaml

publishes:
  # Gateway aggregated API
  - name: com.company.gateway/public-api
    inputPath: specs/public-api.yaml
    version: 2.0.0
    registry: api-registry
    type: openapi
    description: "Public API gateway specification"
    labels:
      component: api-gateway
      visibility: public
```

### Global Registries (`~/.config/apicurio/registries.yaml`)

```yaml
registries:
  # Production registry
  - name: company-prod
    url: https://registry.company.com
    auth:
      type: bearer
      tokenEnv: COMPANY_REGISTRY_TOKEN

  # Staging registry
  - name: company-staging
    url: https://staging-registry.company.com
    auth:
      type: bearer
      tokenEnv: COMPANY_STAGING_TOKEN

  # Data platform registry
  - name: data-registry
    url: https://data-registry.company.com
    auth:
      type: basic
      username: datauser
      passwordEnv: DATA_REGISTRY_PASSWORD

  # API management registry
  - name: api-registry
    url: https://api-registry.company.com
    auth:
      type: token
      tokenEnv: API_REGISTRY_TOKEN

  # Local development
  - name: local
    url: http://localhost:8080
    auth:
      type: none
```

## Advanced Usage

### Environment-Specific Dependencies

Use environment variables to switch between registries:

```yaml
# apicurioconfig.yaml
dependencies:
  - name: user-service
    groupId: com.company.services
    artifactId: user-service
    version: ^1.0.0
    registry: ${APICURIO_REGISTRY:-local}  # Defaults to local
    outputPath: protos/user-service.proto
```

```bash
# Development
export APICURIO_REGISTRY=local
apicurio pull

# Staging
export APICURIO_REGISTRY=company-staging
apicurio pull

# Production
export APICURIO_REGISTRY=company-prod
apicurio pull
```

### Version Pinning Strategies

Different strategies for version management:

```yaml
dependencies:
  # Exact version for stable APIs
  - name: stable-api
    version: 2.1.5
    # ... other config

  # Patch updates only
  - name: patch-updates
    version: ~1.2.0  # >=1.2.0, <1.3.0
    # ... other config

  # Minor updates allowed
  - name: minor-updates
    version: ^1.2.0  # >=1.2.0, <2.0.0
    # ... other config

  # Range specification
  - name: range-updates
    version: ">=1.1.0, <1.4.0"
    # ... other config
```

### Multi-Team Schema Management

Organize schemas by team and domain:

```yaml
dependencies:
  # User team schemas
  - name: user-service
    groupId: com.company.user
    artifactId: service-api
    version: ^2.0.0
    registry: company-prod
    outputPath: protos/user/service.proto

  - name: user-events
    groupId: com.company.user
    artifactId: domain-events
    version: ^1.5.0
    registry: company-prod
    outputPath: schemas/user/events.avsc

  # Payment team schemas
  - name: payment-service
    groupId: com.company.payment
    artifactId: service-api
    version: ^3.1.0
    registry: company-prod
    outputPath: protos/payment/service.proto

  # Cross-cutting concerns
  - name: common-types
    groupId: com.company.common
    artifactId: types
    version: ^1.0.0
    registry: company-prod
    outputPath: protos/common/types.proto
```

## Publishing Workflows

### Development to Production Pipeline

#### 1. Development Phase

```bash
# Work with local registry during development
export APICURIO_REGISTRY=local
apicurio pull

# Publish to local registry for testing
apicurio publish my-schema
```

#### 2. Staging Phase

```yaml
# Update config for staging
publishes:
  - name: com.company.myteam/my-schema
    inputPath: schemas/my-schema.avsc
    version: 1.1.0-rc.1  # Release candidate
    registry: company-staging
    # ... other config
```

```bash
export COMPANY_STAGING_TOKEN="staging-token"
apicurio publish my-schema
```

#### 3. Production Release

```yaml
# Update for production release
publishes:
  - name: com.company.myteam/my-schema
    inputPath: schemas/my-schema.avsc
    version: 1.1.0  # Final version
    registry: company-prod
    # ... other config
```

```bash
export COMPANY_REGISTRY_TOKEN="prod-token"
apicurio publish my-schema
```

### Schema Evolution

Handle breaking and non-breaking changes:

```yaml
publishes:
  # Non-breaking change - patch version
  - name: com.company.api/user-schema
    inputPath: schemas/user-v1.avsc
    version: 1.2.1
    registry: company-prod
    ifExists: CREATE_VERSION
    description: "Added optional email field"

  # Breaking change - major version
  - name: com.company.api/user-schema
    inputPath: schemas/user-v2.avsc
    version: 2.0.0
    registry: company-prod
    ifExists: CREATE_VERSION
    description: "Renamed 'name' field to 'fullName'"
    labels:
      breaking-change: "true"
      migration-guide: "https://docs.company.com/schema-migration-v2"
```

## Team Collaboration

### Shared Configuration

Use external registries file for team-wide registry definitions:

```bash
# Create team registries file
cat > team-registries.yaml << EOF
registries:
  - name: team-prod
    url: https://team-registry.company.com
    auth:
      type: bearer
      tokenEnv: TEAM_REGISTRY_TOKEN
EOF

# Reference in project config
cat > apicurioconfig.yaml << EOF
externalRegistriesFile: team-registries.yaml
dependencies:
  - name: shared-types
    groupId: com.company.team
    artifactId: shared-types
    version: ^1.0.0
    registry: team-prod
    outputPath: protos/shared-types.proto
EOF
```

### Lock File Management

Best practices for lock files in teams:

```bash
# Always commit lock files
git add apicuriolock.yaml
git commit -m "Update schema dependencies"

# Resolve conflicts by regenerating
git checkout HEAD -- apicuriolock.yaml
apicurio lock
git add apicuriolock.yaml
```

### Code Generation Integration

Integrate with build tools:

```yaml
# Makefile example
.PHONY: schemas
schemas:
	apicurio pull
	buf generate --template buf.gen.yaml protos/

.PHONY: verify-schemas  
verify-schemas:
	apicurio verify
	apicurio doctor
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/schemas.yml
name: Schema Management

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  schema-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Apicurio CLI
        run: |
          curl -L https://github.com/YassineElbouchaibi/apicurio-cli/releases/latest/download/apicurio-linux-x64.tar.gz | tar xz
          sudo mv apicurio /usr/local/bin/
      
      - name: Verify schemas
        env:
          COMPANY_REGISTRY_TOKEN: ${{ secrets.REGISTRY_TOKEN }}
        run: |
          apicurio doctor
          apicurio verify
      
      - name: Check for updates
        env:
          COMPANY_REGISTRY_TOKEN: ${{ secrets.REGISTRY_TOKEN }}
        run: |
          apicurio status

  schema-publish:
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    needs: schema-check
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Apicurio CLI
        run: |
          curl -L https://github.com/YassineElbouchaibi/apicurio-cli/releases/latest/download/apicurio-linux-x64.tar.gz | tar xz
          sudo mv apicurio /usr/local/bin/
      
      - name: Publish schemas
        env:
          COMPANY_REGISTRY_TOKEN: ${{ secrets.REGISTRY_TOKEN }}
        run: |
          apicurio publish
```

### Jenkins Pipeline

```groovy
// Jenkinsfile
pipeline {
    agent any
    
    environment {
        COMPANY_REGISTRY_TOKEN = credentials('registry-token')
    }
    
    stages {
        stage('Install CLI') {
            steps {
                sh '''
                    curl -L https://github.com/YassineElbouchaibi/apicurio-cli/releases/latest/download/apicurio-linux-x64.tar.gz | tar xz
                    chmod +x apicurio
                '''
            }
        }
        
        stage('Verify Schemas') {
            steps {
                sh './apicurio doctor'
                sh './apicurio verify'
            }
        }
        
        stage('Check Status') {
            steps {
                sh './apicurio status'
            }
        }
        
        stage('Publish') {
            when {
                branch 'main'
            }
            steps {
                sh './apicurio publish'
            }
        }
    }
}
```

### Docker Integration

```dockerfile
# Dockerfile for schema management
FROM rust:1.70 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/apicurio /usr/local/bin/

WORKDIR /workspace

ENTRYPOINT ["apicurio"]
```

```bash
# Build and use the container
docker build -t apicurio-cli .

# Run schema operations
docker run -v $(pwd):/workspace \
  -e COMPANY_REGISTRY_TOKEN="$COMPANY_REGISTRY_TOKEN" \
  apicurio-cli pull

docker run -v $(pwd):/workspace \
  -e COMPANY_REGISTRY_TOKEN="$COMPANY_REGISTRY_TOKEN" \
  apicurio-cli publish
```

## Troubleshooting

### Common Issues and Solutions

**Authentication failures:**
```bash
# Check token is set
echo $COMPANY_REGISTRY_TOKEN

# Test manual access
curl -H "Authorization: Bearer $COMPANY_REGISTRY_TOKEN" \
  https://registry.company.com/apis/registry/v3/system/info

# Verify in apicurio
apicurio doctor
```

**Version resolution conflicts:**
```bash
# Check available versions
apicurio status

# Force update to latest
apicurio update

# Check specific dependency
apicurio registry list
```

**Lock file issues:**
```bash
# Regenerate lock file
rm apicuriolock.yaml
apicurio lock

# Verify integrity
apicurio verify
```

This tutorial covers the main usage patterns for the Apicurio CLI. For more advanced scenarios or specific questions, refer to the full documentation or open an issue on the project repository.
