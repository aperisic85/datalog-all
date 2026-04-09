-- Fix admin password hash (admin123)
UPDATE users
SET password_hash = '$2b$12$iUbD1mrTvNe4EQpML3mFv.g2MVTbgW6u2T4hKjA6AbOn3z9fvVF1m'
WHERE username = 'admin';
