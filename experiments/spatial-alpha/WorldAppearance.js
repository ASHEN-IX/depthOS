.pragma library

const categories = ["projects", "research", "social", "focus"];
const symbols = ["◈", "◇", "✦", "◎"];

function category(world, index) {
    return world && categories.includes(world.artCategory) ? world.artCategory : categories[index % categories.length];
}

function symbol(category) {
    return symbols[Math.max(0, categories.indexOf(category))];
}

function icon(category) {
    return category === "projects" ? "layers" : category;
}
