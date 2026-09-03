-- ALTER TABLE Groups DROP CONSTRAINT group_index_fk;
DROP TABLE IF EXISTS group_index;
-- ALTER TABLE Blogs DROP CONSTRAINT blog_index_fk;
DROP TABLE IF EXISTS Blog_index;
-- ALTER TABLE Stories DROP CONSTRAINT story_index_fk;
DROP TABLE IF EXISTS Story_index;
-- ALTER TABLE Authors DROP CONSTRAINT author_index_fk;
DROP TABLE IF EXISTS Author_index;

DROP TYPE IF EXISTS fimfic_visibility;
DROP TYPE IF EXISTS archive_status;
