local actions = require('telescope.actions')
local action_state = require('telescope.actions.state')
local finders = require('telescope.finders')
local pickers = require('telescope.pickers')
local sorters = require('telescope.sorters')

function mbox_picker(mboxes)
  pickers.new {
    results_title = 'Mailboxes',
    finder = finders.new_table(mboxes),
    sorter = sorters.fuzzy_with_index_bias(),
    attach_mappings = function(prompt_bufnr)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        vim.fn['himalaya#mbox#post_input'](selection.display)
      end)

      return true
    end,
  }:find()
end
