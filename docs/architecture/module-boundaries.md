# Module Boundaries & Crate Architecture

This document defines module boundaries, dependency flow directions, and cross-module interaction rules.

## 1. Core Principles

1. **Unidirectional Dependency Flow**: Upper layers (Server / Application) may depend on lower layers (Domain / Storage), never cyclic dependencies.
2. **No Direct Table Tampering Across Modules**: All cross-module data access must pass through Service / Repository interfaces or explicit data contracts.
3. **Change Atomicity**: Breaking changes must provide backward compatibility or structured version migration plans.
