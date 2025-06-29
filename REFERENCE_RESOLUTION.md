# Advanced Reference Resolution

This document explains the enhanced reference resolution features that automatically resolve and lock transitive dependencies.

## Overview

When you add a dependency that references other artifacts, those references are automatically discovered and added to the lock file. This ensures that all transitive dependencies are available when you run `apicurio pull`.

## Basic Configuration

```yaml
referenceResolution:
  enabled: true                    # Enable/disable globally
  outputPattern: "references/{groupId}/{artifactId.path}/{artifactId.lastLowercase}.{ext}"
  maxDepth: 5                      # Prevent infinite recursion
```

## Pattern Variables

The `outputPattern` supports various substitution variables:

### Basic Variables
- `{groupId}` - The group ID (e.g., "nprod")
- `{artifactId}` - The full artifact ID (e.g., "sp.frame.Frame")  
- `{version}` - The resolved version (e.g., "4.3.1")
- `{ext}` - File extension based on artifact type (e.g., "proto")

### Advanced Artifact ID Transformations
- `{artifactId.path}` - Converts dots to path separators, excluding last part (`sp.frame.Frame` → `sp/frame`)
- `{artifactId.fullPath}` - Converts dots to path separators, including last part (`sp.frame.Frame` → `sp/frame/Frame`)
- `{artifactId.last}` - Last part after final dot (`sp.frame.Frame` → `Frame`)
- `{artifactId.lastLowercase}` - Last part in lowercase (`sp.frame.Frame` → `frame`)
- `{artifactId.snake_case}` - Snake case version (`sp.frame.Frame` → `sp_frame_frame`)
- `{artifactId.kebab_case}` - Kebab case version (`sp.frame.Frame` → `sp-frame-frame`)
- `{artifactId.lowercase}` - Full lowercase (`sp.frame.Frame` → `sp.frame.frame`)

### Indexed Parts
- `{artifactParts[0]}` - First part (`sp.frame.Frame` → `sp`)
- `{artifactParts[1]}` - Second part (`sp.frame.Frame` → `frame`)
- `{artifactParts[2]}` - Third part (`sp.frame.Frame` → `Frame`)

### Path vs FullPath Examples

For artifact ID `sp.frame.Frame`:

| Variable | Result | Use Case |
|----------|--------|-----------|
| `{artifactId.path}` | `sp/frame` | When you want a directory structure without the class name |
| `{artifactId.fullPath}` | `sp/frame/Frame` | When you want the complete path including the class name |
| `{artifactId.lastLowercase}` | `frame` | For the filename (typically lowercase) |

**Pattern Examples:**
- `"protos/{artifactId.path}/{artifactId.lastLowercase}.{ext}"` → `protos/sp/frame/frame.proto`
- `"schemas/{artifactId.fullPath}.{ext}"` → `schemas/sp/frame/Frame.avsc`

## Output Overrides

For complex naming schemes that don't fit patterns, use explicit overrides:

```yaml
referenceResolution:
  outputOverrides:
    # Registry-specific (highest priority)
    "nprod-apicurio:nprod/sp.frame.Frame": "protos/sp/frame/frame.{ext}"
    
    # Group-level (fallback)
    "nprod/sp.frame.Frame": "protos/sp/frame/frame.{ext}"
    
    # Exclude specific artifacts from resolution (set to null)
    "nprod/sp.internal.Debug": null
```

Override keys can be:
- `"registry:groupId/artifactId"` - Most specific
- `"groupId/artifactId"` - Fallback when no registry match

Override values can be:
- A path pattern string (with variable substitution)
- `null` to completely skip resolving this artifact

## Per-Dependency Control

Override reference resolution for specific dependencies:

```yaml
dependencies:
  # Enable reference resolution for this dependency
  - name: nprod/sp.redd.v1.ReddAPI
    version: 1.0.0
    registry: nprod-apicurio
    outputPath: protos/sp/redd/v1/redd_api.proto
    resolveReferences: true   # Override global setting
    
  # Disable reference resolution for this dependency  
  - name: nprod/standalone.service
    version: 2.0.0
    registry: nprod-apicurio
    outputPath: protos/standalone/service.proto
    resolveReferences: false  # Skip references even if globally enabled
```

## Example Transformation

With the configuration:
```yaml
referenceResolution:
  outputPattern: "protos/{artifactId.path}/{artifactId.lastLowercase}.{ext}"
  outputOverrides:
    "nprod/sp.frame.Frame": "protos/sp/frame/frame.{ext}"
    "nprod/sp.internal.Debug": null  # Skip this artifact
```

The artifact `nprod/sp.frame.Frame` version `4.3.1` becomes:
- **Without override**: `protos/sp/frame/frame.proto` (using `{artifactId.path}` = `sp/frame`)
- **With override**: `protos/sp/frame/frame.proto`
- **If mapped to null**: Artifact is completely skipped and not included in lock file

## Lock File Output

The lock file will contain both direct and transitive dependencies:

```yaml
lockedDependencies:
  # Direct dependency
  - name: nprod/sp.redd.v1.ReddAPI
    registry: nprod-apicurio
    resolvedVersion: "1.0.0"
    outputPath: protos/sp/redd/v1/redd_api.proto
    isTransitive: false
    
  # Transitive dependency (resolved from references)
  - name: nprod/sp.frame.Frame  
    registry: nprod-apicurio
    resolvedVersion: "4.3.1"
    outputPath: protos/sp/frame/frame.proto
    isTransitive: true
```

## Best Practices

1. **Use patterns** for consistent naming schemes
2. **Use overrides** for special cases that don't fit patterns
3. **Set per-dependency controls** when some artifacts shouldn't resolve references
4. **Keep max depth reasonable** (5 is usually sufficient)
5. **Test your patterns** with `apicurio lock --dry-run` (if implemented)

## Migration

Existing configurations continue to work. New features are opt-in:
- `outputOverrides` defaults to empty
- `resolveReferences` per-dependency defaults to global setting
- Advanced pattern variables are optional - basic patterns still work
