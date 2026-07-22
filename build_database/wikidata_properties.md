# Wikidata Properties Cheat Sheet

When parsing a Wikidata JSON dump, the facts inside the `claims` object are keyed by these **P-numbers** (Properties). 

## 🌟 The Absolute Essentials
If you only extract a few properties, make it these. They define the core structure of the knowledge graph.

| ID | Property Name | What it tells you |
| :--- | :--- | :--- |
| **P31** | **instance of** | **The most important property.** Tells you what the item *is* (e.g., human, city, film, chemical element). |
| **P279** | **subclass of** | How categories relate to each other (e.g., a "dog" is a subclass of "canine"). |
| **P18** | **image** | The filename of the item's primary image on Wikimedia Commons. |
| **P856** | **official website** | The official URL associated with the item. |
| **P580** | **start time** | When an event, empire, or term of office began. |
| **P582** | **end time** | When an event, empire, or term of office ended. |

---

## 👤 People & Biographies
Crucial for extracting data about historical figures, celebrities, and politicians.

| ID | Property Name | What it tells you |
| :--- | :--- | :--- |
| **P569** | **date of birth** | The exact birth date (often includes timezone/calendar qualifiers). |
| **P570** | **date of death** | The exact death date. |
| **P19** | **place of birth** | Links to the Q-ID of the city or hospital where they were born. |
| **P21** | **sex or gender** | The biological sex or gender identity of the person. |
| **P106** | **occupation** | What they do for a living (e.g., politician, physicist, actor). |
| **P27** | **country of citizenship** | The country they legally belong to. |
| **P1477**| **birth name** | Their original name if they changed it or use a stage name. |

---

## 🌍 Geography & Places
Used for mapping, spatial analysis, and demographics.

| ID | Property Name | What it tells you |
| :--- | :--- | :--- |
| **P625** | **coordinate location** | Latitude and longitude (stored as a specific `globecoordinate` data type). |
| **P17** | **country** | Which sovereign state this place currently belongs to. |
| **P131** | **located in the admin entity**| The state, province, or county the city is inside. |
| **P1082**| **population** | Number of inhabitants (often has a qualifier for *when* the census was taken). |
| **P2046**| **area** | The physical size of the place (usually in square kilometers). |
| **P421** | **time zone** | The timezone the place operates in. |

---

## 🎬 Media, Art, & Science
Useful for databases of books, movies, games, and research.

| ID | Property Name | What it tells you |
| :--- | :--- | :--- |
| **P50** | **author** | The creator of a written work. |
| **P57** | **director** | The director of a film or television episode. |
| **P161** | **cast member** | Actors in a production. |
| **P136** | **genre** | The stylistic category (e.g., science fiction, jazz). |
| **P577** | **publication date** | When the media was released. |

---

## 🧐 Interesting, Niche, & Fun Properties
These properties might not be necessary for a basic database, but they show the incredible depth of Wikidata and are great for building trivia apps, complex queries, or finding weird data connections.

| ID | Property Name | Why it's interesting |
| :--- | :--- | :--- |
| **P1441**| **present in work** | Links a fictional character (e.g., Darth Vader) to the movies or books they appear in. |
| **P2534**| **defining formula** | Stores the actual mathematical/physics formula (in LaTeX) that defines a scientific concept (e.g., $E=mc^2$). |
| **P395** | **licence plate code** | The letters on a license plate for a specific city or region. |
| **P1532**| **country for sport** | Often different from citizenship! (e.g., Scottish players representing Scotland instead of the UK). |
| **P282** | **writing system** | The alphabet or script used by a specific language (e.g., Cyrillic, Kanji). |
| **P1050**| **medical condition** | Diseases or conditions associated with a person (e.g., Abraham Lincoln -> Marfan syndrome). |
| **P460** | **said to be the same as** | Used for things that *might* be the exact same historical entity, but scholars are still debating it. |
| **P3828**| **wears** | Iconic clothing items associated with an item (often used for cartoon characters or historical uniforms). |
