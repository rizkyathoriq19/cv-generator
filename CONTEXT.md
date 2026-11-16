# CV Generator

An application for individual job seekers to prepare CVs for job applications.

## Language

**CV**:
A job seeker's career document containing selected qualifications and career information for an application.
_Avoid_: Resume as a separate product concept.

**Project**:
One independently editable CV, including its content language and selected content structure.
_Avoid_: Master profile, collection of synchronized CVs.

**CV copy**:
An independent project created by duplicating another CV, for example for another job application or language. Later edits do not propagate between copies.
_Avoid_: Linked variant, synchronized translation.

**Content language**:
The language of a CV's authored content and standard section headings, either Indonesian or English. It is distinct from the application's interface language.

**Local library**:
The collection of independently editable CV projects managed by the application on the user's device, distinct from separately exported project backups.

**CV date**:
A career date with month/year precision, year-only precision, or no supplied value. Present denotes an ongoing role rather than an inferred end date.

**Skill group**:
A named capability supported by user-authored achievement bullets, used as leading content in functional and combination CVs.

**Content structure**:
The way CV information is organized, independently of its ATS-friendly presentation.
_Avoid_: Format when it conflates content organization, visual presentation, and file type.

**Reverse chronological**:
A content structure that presents dated career entries from most recent to oldest.

**Functional**:
A content structure that emphasizes skills and related evidence rather than using employment chronology as its main narrative.

**Combination**:
A content structure that combines skill-focused content with a reverse chronological employment history.

**ATS-friendly presentation**:
Conservative document presentation intended to support applicant tracking system parsing. It is not a guarantee of acceptance or identical parsing across systems.
_Avoid_: ATS-certified, guaranteed ATS pass.

**PDF export**:
A generated, text-based CV document intended for submission or sharing, distinct from the editable project backup.

**Project backup**:
A portable snapshot of editable CV data that imports as an independent CV without overwriting an existing project. It is not an editable Word document.

**Trash**:
A local collection of deleted CVs that remain restorable until explicitly and permanently deleted. It is not secure erasure.
