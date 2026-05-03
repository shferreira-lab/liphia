# test_api.ps1
# Testa a API REST Liphia em sequencia
# Rode em outra aba do PowerShell enquanto a API está rodando:
#   .\test_api.ps1

$base = "http://localhost:3000"

function Show($label, $result) {
    Write-Host ""
    Write-Host "── $label" -ForegroundColor Cyan
    Write-Host $result
}

# 1. health (banco vazio)
Show "GET /health" (curl -s "$base/health")

# 2. lista vazia
Show "GET /users (vazio)" (curl -s "$base/users")

# 3. criar Alice
Show "POST /users Alice" (
    curl -s -X POST "$base/users" `
        -H "Content-Type: application/json" `
        -d '{"name":"Alice","email":"alice@test.com"}'
)

# 4. criar Bob
Show "POST /users Bob" (
    curl -s -X POST "$base/users" `
        -H "Content-Type: application/json" `
        -d '{"name":"Bob","email":"bob@test.com"}'
)

# 5. criar Carol
Show "POST /users Carol" (
    curl -s -X POST "$base/users" `
        -H "Content-Type: application/json" `
        -d '{"name":"Carol","email":"carol@test.com"}'
)

# 6. listar todos
Show "GET /users (3 users)" (curl -s "$base/users")

# 7. buscar user 1
Show "GET /users/1" (curl -s "$base/users/1")

# 8. buscar user inexistente
Show "GET /users/999 (not found)" (curl -s "$base/users/999")

# 9. health com users
Show "GET /health (com users)" (curl -s "$base/health")

# 10. deletar user 2
Show "DELETE /users/2" (curl -s -X DELETE "$base/users/2" -w "HTTP %{http_code}")

# 11. listar apos delete
Show "GET /users (apos delete)" (curl -s "$base/users")

# 12. rota inexistente
Show "GET /unknown (404)" (curl -s "$base/unknown")

# 13. POST sem body
Show "POST /users sem body (400)" (
    curl -s -X POST "$base/users" -H "Content-Type: application/json" -d ""
)

Write-Host ""
Write-Host "── Testes concluidos" -ForegroundColor Green
