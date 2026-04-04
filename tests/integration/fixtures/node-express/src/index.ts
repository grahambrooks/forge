import express from 'express';

const app = express();

app.get('/users', (req, res) => {
  res.json([]);
});

app.post('/users', (req, res) => {
  res.status(201).json({});
});

app.get('/health', (req, res) => {
  res.send('ok');
});

const dbUrl = 'postgres://localhost:5432/users';

app.listen(3000);
