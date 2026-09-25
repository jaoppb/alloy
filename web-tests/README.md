# Alloy Web Test Suite (`web-tests/`)

Banco de testes HTTP locais para validação de conformidade dos subsistemas `core/html` e `core/css` do **Alloy**.

Esta suíte foi concebida para ser **100% autocontida, offline e determinística**, sem qualquer dependência de recursos
externos (sem Google Fonts, sem CDNs externas e sem scripts de terceiros).

---

## 🚀 Como Iniciar o Servidor de Testes

O servidor web é executado utilizando **Caddy** em container Docker via **Docker Compose**:

### Pelo `justfile` (Recomendado)

```bash
# Inicia o servidor Caddy em segundo plano
just serve-tests

# Executa o smoke test automático (HTTP 200 + renderização Alloy)
just test-web

# Encerra e remove o container
just stop-tests
```

### Manualmente com Docker Compose

```bash
# Iniciar na pasta web-tests/
cd web-tests && docker compose up -d

# Visualizar logs
docker compose logs -f

# Parar
docker compose down
```

Uma vez iniciado, a suíte estará disponível em: **`http://localhost:8080`**

---

## 📂 Organização dos Testes

- **`index.html`**: Painel central de navegação com catálogo categorizado de todos os testes disponíveis.
- **`assets/`**:
    - `base.css`: Estilos base compatíveis com o subconjunto v0.5 do Alloy.
    - `test-pattern.png`: Imagem PNG padrão para testes de carregamento de imagens locais.
- **`html/`**:
    - `semantic-tags.html`: Tags estruturais (`article`, `section`, `nav`, `header`, `footer`, `main`, etc.).
    - `text-formatting.html`: Elementos inline e bloco de código (`strong`, `em`, `code`, `pre`, `span`, `br`, `hr`).
    - `lists-omission.html`: Listas ordenadas e não ordenadas com teste de omissão de fechamento de `<li>`.
    - `entities-syntax.html`: Entidades nomeadas (`&copy;`, etc.), numéricas, atributos com/sem aspas e comentários.
    - `graceful-degradation.html`: Resiliência diante de tags customizadas ou fora do corte v0.5.
- **`css/`**:
    - `box-model/`: Testes de margens colapsadas (`margin-collapse.html`), padding e border-width
      (`padding-border.html`) e dimensionamento (`box-sizing.html`).
    - `flexbox/`: Direção e alinhamento principal (`direction-justify.html`), eixo cruzado (`align-items-self.html`),
      proporções (`grow-shrink-basis.html`) e quebra de linhas (`wrap.html`).
    - `inline-text/`: Tratamento de espaços e quebras (`white-space.html`) e alinhamento (`text-align.html`).
    - `selectors/`: Combinadores (`combinators.html`), pseudo-classes estruturais (`pseudo-classes.html`) e
      cascata/especificidade (`cascade.html`).
    - `media-queries/`: Regras responsivas de viewport (`min-max-width.html`).
    - `graceful-degradation.html`: Descarte seguro de propriedades desconhecidas via `ParseNote`.
- **`showcase/`**:
    - `card-component.html`: Componente de cartão UI realista com flexbox e imagem local.
    - `article-page.html`: Artigo de blog completo com tipografia e fluxo de texto.
- **`navigation/`**:
    - `page-a.html` & `page-b.html`: Fluxo de navegação cruzada entre páginas via clique em links.
    - `link-resolution.html`: Resolução de URLs relativas, absolutas, subindo pastas (`..`), query strings e âncoras.
    - `error-404.html`: Tratamento e resposta a links quebrados ou rotas inexistentes (404).

---

## 🎯 Convenções dos Testes (Pass / Fail)

Cada página de teste inclui um cabeçalho padrão com:

1. **Identificador da especificação**: Qual padrão W3C e requisito do Alloy está sendo verificado.
2. **Critério PASS (Verde)**: Descrição exata da geometria, cores ou posicionamento esperado.
3. **Critério FAIL (Vermelho)**: Indicador visual em caso de regressão ou cálculo incorreto de layout.
