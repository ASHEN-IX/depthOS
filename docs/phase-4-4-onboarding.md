# Phase 4.4 First-Run Onboarding Wizard

DepthOS Phase 4.4 includes a dedicated 6-step Onboarding Wizard (`OnboardingWizard.qml`) that activates when launching DepthOS without an existing workspace database or on user request.

## Mental Model Education

The primary objective of the wizard is to prevent user confusion between **DepthOS Logical Desktops** and **Native KDE Virtual Desktops**:

1. **DepthOS Logical Desktops**: Application-owned task contexts (e.g. "Review", "Development", "Compiler") organizing projects and observed window associations.
2. **KDE Native Desktops**: External window manager workspaces managed by KWin. DepthOS observes native windows on KDE desktops in a strictly read-only manner.

## 6-Step Guided Flow

```
Step 1: Welcome & Architectural Overview
  └── Step 2: Establish Your First World (Name & Artwork Motif)
        └── Step 3: Configure Primary Workspace (Project Directory Root)
              └── Step 4: Define Logical Desktops (Specialized Roles)
                    └── Step 5: Presentation & Motion Preferences
                          └── Step 6: Review & Atomic Initialization
```

- **Step 1: Welcome & Architecture**: Explains the 3-tier structure (`World` → `Workspace` → `Logical Desktop`) and clarifies that native KWin state remains external and read-only.
- **Step 2: World Setup**: Guides creation of the initial World (default: "Primary Operations"), with tone selection and cyberpunk artwork preview.
- **Step 3: Workspace & Project Root**: Prompts for the first Workspace name (default: "Core Project") and allows associating an existing local Git directory.
- **Step 4: Logical Desktop Roles**: Offers one-click selection of initial task desktop presets (`Code`, `Review`, `Terminal`).
- **Step 5: Preferences**: Configures visual preferences: Reduced Motion (for accessibility or performance), Low Effects (disabling heavy glows), and Show Window Titles.
- **Step 6: Confirm & Launch**: Displays a summary of the hierarchy to be generated. Clicking "Initialize & Launch" executes an atomic transaction creating the database, initial records, and preferences in one step.
