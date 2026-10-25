-- A base de estudos passa a ser exclusivamente de questões oficiais do ENEM.
-- Questões autorais de demonstração são removidas; tentativas e comentários
-- associados são removidos em cascata.
DELETE FROM questions
WHERE external_id IS NULL;
