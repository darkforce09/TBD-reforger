-- Every accepted save of a wiki page is a numbered revision. `wiki_pages.revision` is the number of
-- the page's current content; `wiki_page_revisions` keeps one immutable copy of the page for every
-- revision, written in the same transaction as the page update it records. A save names the
-- revision it edits, so two editors cannot overwrite each other unnoticed.
ALTER TABLE public.wiki_pages
    ADD COLUMN revision integer NOT NULL DEFAULT 1 CHECK (revision > 0);

-- `author_id` names a member by Discord id and, like every other actor stamp, carries no foreign
-- key. Deleting a page deletes its history.
CREATE TABLE public.wiki_page_revisions (
    page_id uuid NOT NULL REFERENCES public.wiki_pages(id) ON DELETE CASCADE,
    revision integer NOT NULL CHECK (revision > 0),
    slug text NOT NULL,
    category text NOT NULL,
    title text NOT NULL,
    icon text,
    nav_order bigint NOT NULL,
    body_md text NOT NULL,
    author_id text,
    created_at timestamptz DEFAULT now(),
    PRIMARY KEY (page_id, revision)
);

-- Every existing page starts its history at revision 1: its current content, attributed to its
-- last editor at the time of its last edit. Either stays null when the page does not record it.
INSERT INTO public.wiki_page_revisions
    (page_id, revision, slug, category, title, icon, nav_order, body_md, author_id, created_at)
SELECT id, 1, slug, category, title, icon, nav_order, body_md, updated_by, updated_at
FROM public.wiki_pages;
