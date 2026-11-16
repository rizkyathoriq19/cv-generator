# Store working CVs as individual local project files

Use an application-managed local library with one versioned JSON file per CV rather than a database or user-selected working-file locations. Independently owned CVs do not require relational storage; autosave with atomic replacement protects existing files, while separately exported project backups provide portability. File schema compatibility, crash recovery, and safe import remain explicit design obligations, not reasons to silently introduce a database.
