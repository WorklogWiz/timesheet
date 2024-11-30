```mermaid
---
title: Animal example
---
classDiagram
    class MainProgram

    namespace lib {
        class Journal
        class Entry
        class Config
        class date
    }
    
    namespace journal {
        class JournalSql
        class JournalCsv
    }
    
```