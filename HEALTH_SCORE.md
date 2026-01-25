# Scenery Health Score

The **Health Score** is a diagnostic tool in X-Addon-Oxide that ensures your scenery packages are correctly structured and classified. It helps identify "ghost" scenery, empty folders, or misclassified addons that might not work as expected in X-Plane.

## How it's Calculated

The score is out of 100% and is based on three main criteria:

### 1. Classification (20 points)

A package gets 20 points simply by being successfully categorized (e.g., as an "Airport," "Ortho," or "Library") based on its folder name or content patterns.

### 2. Content Matching (Max 60 points)

This is the most critical check. The system verifies that the package actually contains the data it claims to have:

- **Airports**: Must contain at least one `apt.dat` file.
- **Ortho/Mesh/Overlays**: Must contain folder structures like `Earth nav data` (tiles).
- **Libraries**: System-wide libraries are automatically given full credit if they follow proper naming conventions.

### 3. User Metadata (10 points)

Packages that include custom user tags get a 10% bonus. This indicates the pack has been manually verified or organized by the user.

---

## 90% vs. 100%: Which is Better?

A common question is whether you should "chase" a 100% score.

**Short Answer: No. 90% is the perfect score for a standard addon.**

- **90% (Excellent)**: This is the practical ceiling for custom airports, scenery, and overlays. It means the pack is correctly categorized, contains the right data, and has been tagged.
- **100% (System)**: This score is reserved for "Trusted System Packs" like *Global Airports*, *X-Plane Demo Areas*, and *Core Libraries*. These get a +10 point "Trust Bonus" because they are essential to X-Plane's core functionality.

If your favorite airport is at 90%, it is performing perfectly!

---

## Status Indicators

| Label | Score | Meaning |
|-------|-------|---------|
| **EXCELLENT** | 90 - 100% | Properly structured and perfectly categorized. |
| **STABLE** | 70 - 89% | Valid content found, though it might be a lightweight addon or missing some metadata. |
| **NEEDS ATTENTION** | 40 - 69% | **Mismatch detected.** For example, an "Airport" pack that contains no airports. |
| **CRITICAL** | < 40% | Likely an empty folder, a corrupt install, or a non-scenery folder in your Custom Scenery directory. |

---

## Toggling Visibility

If you are a power user or simply prefer a cleaner interface, you can hide these scores:

1. Navigate to the **Scenery** tab.
2. Open the **Map Filter** menu.
3. Find the **Utilities** section.
4. Uncheck **Scenery Health Scores**.

This will hide the health indicator from the **Inspector Panel** while keeping the underlying sorting logic intact.
